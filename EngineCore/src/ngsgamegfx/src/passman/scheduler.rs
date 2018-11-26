//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use atomic_refcell::AtomicRefCell;
use std::{
    cell::{Cell, RefCell},
    ops::Range,
    sync::Arc,
};

use zangfx::base as gfx;

use super::{Pass, PassInfo, Resource, ResourceId, ResourceInfo, ResourceRef};
use crate::utils::{
    any::AsAnySendSync,
    iterator_mut::{IteratorMut, IteratorToIteratorMutExt},
};

#[cfg(test)]
#[path = "./scheduler_test.rs"]
mod scheduler_test;

/// `ResourceInfo` with type-erased `Self::Resource`.
///
/// Has a blanket implementation for every `T: ResourceInfo`.
pub trait UntypedResourceInfo: AsAnySendSync + std::fmt::Debug {
    fn build_untyped(
        &self,
        context: &ResourceInstantiationContext<'_>,
    ) -> gfx::Result<Box<dyn Resource>>;
}

impl<T: ResourceInfo> UntypedResourceInfo for T {
    fn build_untyped(
        &self,
        context: &ResourceInstantiationContext<'_>,
    ) -> gfx::Result<Box<dyn Resource>> {
        self.build(context).map(|x| x as Box<dyn Resource>) // unsize `Self::Resource` to `dyn Resource`
    }
}

/// Stores the description of a pass graph and serves as a builder object of
/// [`Schedule`].
#[derive(Debug)]
pub struct ScheduleBuilder<C: ?Sized> {
    resources: Vec<BuilderResource>,
    passes: Vec<BuilderPass<C>>,
}

#[derive(Debug)]
struct BuilderPass<C: ?Sized> {
    info: Option<PassInfo<C>>,

    // The rest of the fields are used as a temporary storage for
    // `ScheduleBuilder::schedule`
    scheduled: bool,

    /// Indices into `pass_order`, each indicating a pass dependent on the
    /// output of the current pass.
    next_passes: RefCell<Vec<usize>>,

    /// Indices into `pass_order`, each indicating a pass the current pass is
    /// dependent on the output of.
    previous_passes: Vec<usize>,

    /// Nonce token that can be used to define a set membership.
    token: Cell<usize>,

    /// Indicates whether this is an output pass. See [`RunnerPass::output`].
    output: bool,
}

impl<C: ?Sized> BuilderPass<C> {
    /// Unwrap `self.info`.
    fn info(&self) -> &PassInfo<C> {
        self.info.as_ref().unwrap()
    }
}

impl<C: ?Sized> From<PassInfo<C>> for BuilderPass<C> {
    fn from(x: PassInfo<C>) -> Self {
        Self {
            info: Some(x),
            scheduled: false,
            next_passes: RefCell::new(Vec::new()),
            previous_passes: Vec::new(),
            token: Cell::new(0),
            output: false,
        }
    }
}

#[derive(Debug)]
struct BuilderResource {
    object: Box<dyn UntypedResourceInfo>,

    // The rest of the fields are used as a temporary storage for
    // `ScheduleBuilder::schedule`
    is_output: bool,

    /// The number of consuming uses of this resource originating from
    /// unscheduled passes.
    num_consuming_passes: usize,

    /// An index into `pass_order` plus one, indicating the earliest executed
    /// pass that consumes this resource.
    earliest_consuming_pass_index: usize,

    /// A range into `pass_order`.
    lifetime: Range<usize>,

    aliasable: bool,
}

impl From<Box<dyn UntypedResourceInfo>> for BuilderResource {
    fn from(x: Box<dyn UntypedResourceInfo>) -> Self {
        Self {
            object: x,
            is_output: false,
            num_consuming_passes: 0,
            earliest_consuming_pass_index: 0,
            lifetime: 0..0,
            aliasable: true,
        }
    }
}

impl<C: ?Sized> ScheduleBuilder<C> {
    pub fn new() -> Self {
        Self {
            resources: Vec::new(),
            passes: Vec::new(),
        }
    }

    /// Define a `ResourceInfo`.
    ///
    /// Returns the `ResourceRef` representing the newly defined
    /// resource. The returned `ResourceRef` only pertains to `self`.
    pub fn define_resource<T: ResourceInfo>(&mut self, resource: T) -> ResourceRef<T> {
        let next_index = self.resources.len();
        self.resources
            .push((Box::new(resource) as Box<dyn UntypedResourceInfo>).into());
        ResourceRef::new(ResourceId(next_index))
    }

    /// Mutably borrow a `ResourceInfo` specified by `id`.
    pub fn get_resource_info_mut<T: ResourceInfo>(&mut self, id: ResourceRef<T>) -> &mut T {
        ((*self.resources[id.0].object).as_any_mut())
            .downcast_mut()
            .expect("type mismatch")
    }

    pub fn define_pass(&mut self, pass: PassInfo<C>) {
        self.passes.push(pass.into());
    }

    /// Construct a `Schedule` consuming `self`.
    ///
    /// # Panics
    ///
    /// Will panic if there exists no ordering of passes that agrees with
    /// their resource dependencies.
    ///
    pub fn schedule(mut self, output_resources: &[&ResourceId]) -> Schedule<C> {
        use std::mem::replace;

        // Nonce token
        let mut token = 0;

        // Fill out the field `is_output`
        for resource in output_resources {
            self.resources[resource.0].is_output = true;
        }

        // First, find an ordering of passes that agrees with a partial order
        // `⋖` defined by resource produce-consume dependencies.
        // (We call this *chronological order*.)
        // Since there are many such possible orderings, we seek to find the one
        // that minimizes the GPU stall time by employing a greedy algorithm and
        // heuristics.
        // Although this has a potential drawback regarding the cache efficiency,
        // an experiment would be required to confirm its performance impact.

        // The partial order `⋖` is defined as following:
        //
        //  1. If there exists a resource produced by a pass `P` and consumed by
        //     a pass `C`, `P ⋖ C`.
        //  2. If `a ⋖ b && b ⋖ c`, then `a ⋖ c` (transitivity).

        let mut pass_order = Vec::with_capacity(self.passes.len());

        for pass in &self.passes {
            for res_use in &pass.info().resource_uses {
                if !res_use.produce {
                    self.resources[res_use.resource.0].num_consuming_passes += 1;
                }
            }
        }

        // TODO: prune unnecessary passes (i.e., those that don't contribute
        //       to `output_resources`)

        // This loop constructs the ordering in a reverse chronological order.
        loop {
            let mut has_remaining_passes = false;
            let pass_order_prev_len = pass_order.len();

            // Find the maximal elements of the partial order. (They are the
            // last pass(es) to execute.)
            'a: for (i, pass) in self.passes.iter_mut().enumerate() {
                // Exclude already-scheduled passes from the partial order
                if pass.scheduled {
                    continue;
                }

                has_remaining_passes = true;

                for res_use in &pass.info().resource_uses {
                    if res_use.produce
                        && self.resources[res_use.resource.0].num_consuming_passes > 0
                    {
                        // There exists a pass `p` such that `pass ⋖ p`.
                        // Skip this one.
                        continue 'a;
                    }
                }

                if pass_order_prev_len == 0 {
                    // This is an maximal element of the original partial order.
                    pass.output = true;
                }

                pass_order.push(i);
            }

            if !has_remaining_passes {
                break;
            }

            if pass_order_prev_len == pass_order.len() {
                // When this code is reached, it means that we could not make a
                // progress in this iteration. This happens only if the partial
                // order is not actually a partial order (i.e., there is a
                // cyclic dependency) because a non-empty finite partial order
                // is guaranteed to have a maximal element.
                panic!("no valid solution");
            }

            if pass_order.len() > pass_order_prev_len + 1 {
                // There are multiple candidates. Choose the one that minimizes
                // the impact of pipeline stall.

                let (i, _) = pass_order[pass_order_prev_len..]
                    .iter()
                    .enumerate()
                    .min_by_key(|&(_, &i)| {
                        let ref pass = self.passes[i];
                        (pass.info().resource_uses)
                            .iter()
                            .filter_map(|res_use| {
                                let ref res: BuilderResource = self.resources[res_use.resource.0];

                                if res_use.produce {
                                    Some(res.earliest_consuming_pass_index)
                                } else {
                                    None
                                }
                            })
                            .max()
                            .unwrap_or(0)
                    })
                    .unwrap();

                pass_order.swap(i + pass_order_prev_len, pass_order_prev_len);
            }

            // Leave the best candidate and forget about the others
            pass_order.truncate(pass_order_prev_len + 1);

            // `pass_order[pass_order_prev_len]` represents the maximal element
            // that have just been chosen.
            let ref mut pass = self.passes[pass_order[pass_order_prev_len]];

            pass.scheduled = true;

            for res_use in &pass.info().resource_uses {
                if !res_use.produce {
                    let ref mut res: BuilderResource = self.resources[res_use.resource.0];
                    res.num_consuming_passes -= 1;
                    res.earliest_consuming_pass_index = pass_order_prev_len + 1;
                }
            }
        }

        // Turn it into a chronologoical order
        pass_order.reverse();

        // After that, we find a valid lifetime of each resource. The lifetime
        // of a resource is valid only if the following conditions are met:
        //
        //  - It starts before the producing pass.
        //  - It ends after all consuming passes.
        //
        // In order to optimize the run-time performance, we also take memory
        // barriers required to prevent write-after-read (or in this case,
        // produce-after-consume) pipeline hazards into account. Memory barries
        // inserted to prevent such pipeline hazards are called
        // *aliasing barriers*.
        //
        // Aliasing barriers between the uses of two possibly-aliased resources
        // are required only if the resources really occupy an overlapping
        // region. Unfortunately, we don't have a full control over a heap's
        // memory layout and we are limited to an abstract interface of `bind`
        // and `make_aliasable`, so we have to be conservative.
        // On a bright side, this restriction greatly simplifies the reasoning
        // needed here.
        //
        // TODO: Resources having different `ResourceBind::memory_type` thus
        //       allocated to a different heaps never alias
        //
        // A memory barrier must exist between a pass `C` and `P` (`C` is issued
        // earlier than `P` in `pass_order`) if there exists a pair of a
        // resource `c` used by `C` and a resource `p` used by `P` such that the
        // lifetimes of `c` and `p` do not overlap.
        // A trivial solution to this problem is to make all resources stay
        // alive all the time, but it's the worst solution in terms of the
        // memory consumption. Another trivial solution is to use the smallest
        // valid lifetime for every resource, which essentially serializes
        // all passes, hampering the GPU's parallellism.
        //
        // Here's an example showing the case where a trivial solution happens
        // to be the optimal solution (where no extra barriers had to be
        // inserted as aliasing barriers):
        //
        //     A    =========
        //     B           =================    === - smallest valid lifetime
        //     C        ==========
        //     D               =============
        //     E                         ===
        //
        //          A ---> AB ------+--> BDE   ("BDE" represents a pass
        //                          |           using resources B, D, and E)
        //              C ----> CD -+
        //
        // Let's show an evidence of the existence of a non-trivial good
        // solution.
        //
        //     A    =======xxxxxxxxx
        //     B         ====================    === - smallest valid lifetime
        //     C    yyyyyyyyy=======             xxx - extended lifetime 1
        //     D    yyyyyyyyyyyyyy===========    yyy - extended lifetime 2
        //     E                          ===
        //
        //          A -> AB ----------+-> BDE
        //                            |
        //                   C -> CD -+
        //
        // With a trivial solution (where the smallest valid lifetimes are used),
        // an aliasing barrier from `AB` to `C` would be needed. By extending
        // the lifetime of `A`, we could get rid of this extra barrier.
        // Alternatively, we could extend those of `C` and `D` with a similar
        // milage.
        //
        // This presents a trade-off between memory consumption and a potential
        // opportunity of harnessing GPU parallelism. What kind of inputs leads
        // to the worst case with lifetime extension? What can GPU parallelism
        // offer? This trade-off remains to be discussed in the future.

        // Calculate the smallest valid lifetime for each resource.
        for (i, &pass_i) in pass_order.iter().enumerate() {
            let ref mut pass: BuilderPass<C> = self.passes[pass_i];

            for res_use in &pass.info().resource_uses {
                let ref mut res: BuilderResource = self.resources[res_use.resource.0];

                if !res_use.aliasable {
                    res.lifetime = 0..pass_order.len();
                    res.aliasable = false;
                } else if res_use.produce {
                    debug_assert_eq!(res.lifetime, 0..0);
                    res.lifetime = i..i + 1;
                } else {
                    res.lifetime.end = i + 1;
                }
            }
        }

        // Identify pass dependencies. (Fill `next_passes` and `previous_passes`)
        for (i, &pass_i) in pass_order.iter().enumerate() {
            let mut previous_passes = Vec::new();

            // Generate a new `token`
            token += 1;

            for r_use in self.passes[pass_i].info().resource_uses.iter() {
                let ref res: BuilderResource = self.resources[r_use.resource.0];
                if !r_use.produce {
                    assert!(r_use.aliasable);

                    // The minimum valid lifetime of a resource always starts
                    // at the producer
                    let k = res.lifetime.start;

                    let ref pass: BuilderPass<C> = self.passes[pass_order[k]];

                    if pass.token.get() == token {
                        // It's already in `previous_passes`
                        continue;
                    }

                    pass.token.set(token);
                    pass.next_passes.borrow_mut().push(i);
                    previous_passes.push(k);
                }
            }

            self.passes[pass_i].previous_passes = previous_passes;
        }

        // Scan passes in the chronological order
        for (i, &pass_i) in pass_order.iter().enumerate() {
            // Find passes which follow the current one but are unordered
            // to the current one.

            // Generate a new `token`. We use it to mark every pass `k`
            // that satisfies `i ⩿ k`. (`⩿` is a union of `==` and `⋖`)
            token += 1;

            let ref i_pass: BuilderPass<C> = self.passes[pass_i];
            i_pass.token.set(token);

            for k in i..pass_order.len() {
                let ref k_pass: BuilderPass<C> = self.passes[pass_order[k]];
                if k_pass.token.get() != token {
                    // not `i ⩿ k`
                    continue;
                }

                for &m in k_pass.next_passes.borrow().iter() {
                    debug_assert!(m > k);

                    let ref m_pass: BuilderPass<C> = self.passes[pass_order[m]];
                    m_pass.token.set(token);
                }
            }

            // Now, check if there is a pass `k` such that `!(i ⩿ k)`
            // (i.e., no barriers are generated by resource dependency), `k` is
            // chronologically ordered after `i`, and an explicit aliasing
            // barrier is required between `i` and `k`.
            //
            // Extend the lifetimes of resources used by `i` to eliminate
            // any such explicit aliasing barriers.
            let i_lifetime_end_min = (i_pass.info().resource_uses)
                .iter()
                .map(|res_use| self.resources[res_use.resource.0].lifetime.end)
                .min()
                .unwrap_or(pass_order.len());

            let mut new_i_lifetime_end_min = i_lifetime_end_min;

            for k in i_lifetime_end_min..pass_order.len() {
                let ref k_pass: BuilderPass<C> = self.passes[pass_order[k]];
                if k_pass.token.get() == token {
                    // `i ⩿ k`
                    continue;
                }

                new_i_lifetime_end_min = k + 1;
            }

            use std::cmp::max;

            for res_use in &i_pass.info().resource_uses {
                let ref mut res: BuilderResource = self.resources[res_use.resource.0];
                res.lifetime.end = max(res.lifetime.end, new_i_lifetime_end_min);
            }
        }

        // TODO: Prune redundant barriers. See the following example:
        // We have resource dependencies `A -> B`, `B -> C`, and `A -> C`.
        // In this case, the barrier `A -> C` is redundant because there is
        // another path from `A` to `C` that goes through `B`.

        // Now, we have everything we need to construct `Schedule`

        let mut proto_passes: Vec<_> = pass_order
            .iter()
            .map(|&i| {
                let ref mut pass: BuilderPass<C> = self.passes[i];
                SchedulePass {
                    info: pass.info.take().unwrap(),
                    wait_on_passes: replace(&mut pass.previous_passes, Vec::new()),
                    bind_resources: Vec::new(),
                    unbind_resources: Vec::new(),
                    output: pass.output,
                }
            })
            .collect();

        // TODO: Segregate unaliasable resources. We could leverage the dedicated
        // allocation extension of Vulkan.

        for (i, res) in self.resources.iter().enumerate() {
            if res.lifetime.end > res.lifetime.start {
                proto_passes[res.lifetime.start].bind_resources.push(i);
                proto_passes[res.lifetime.end - 1].unbind_resources.push(i);
            }
        }

        let proto_resources = self.resources.drain(..).map(|r| r.object).collect();

        Schedule {
            passes: proto_passes,
            resources: proto_resources,
        }
    }
}

/// A compiled execution plan of a pass graph.
#[derive(Debug)]
pub struct Schedule<C: ?Sized> {
    passes: Vec<SchedulePass<C>>,
    resources: Vec<Box<dyn UntypedResourceInfo>>,
}

#[derive(Debug)]
struct SchedulePass<C: ?Sized> {
    info: PassInfo<C>,
    wait_on_passes: Vec<usize>,
    bind_resources: Vec<usize>,
    unbind_resources: Vec<usize>,
    output: bool,
}

#[derive(Debug)]
pub struct ResourceInstantiationContext<'a> {
    device: &'a gfx::DeviceRef,
    queue: &'a gfx::CmdQueueRef,
}

#[derive(Debug)]
pub struct PassInstantiationContext<'a> {
    resources: &'a [Arc<dyn Resource>],
}

impl<C: ?Sized> Schedule<C> {
    /// Construct a `ScheduleRunner` by allocating device resources required for
    /// the execution of the plan.
    pub fn instantiate(
        self,
        device: &gfx::DeviceRef,
        queue: &gfx::CmdQueueRef,
    ) -> gfx::Result<ScheduleRunner<C>> {
        let mut heap_builders = vec![None; 32];

        // Instantiate resources
        let context = ResourceInstantiationContext { device, queue };
        let resources: Vec<Arc<dyn Resource>> = self
            .resources
            .into_iter()
            .map(|r| {
                let boxed = r.build_untyped(&context)?;
                Ok(boxed.into()) // convert `Box` into `Arc`
            })
            .collect::<gfx::Result<_>>()?;

        // Bind GFX resources
        for pass in &self.passes {
            for &i in &pass.bind_resources {
                let ref resource = resources[i];
                if let Some(resource_bind) = resource.resource_bind() {
                    let ref mut hb_cell = heap_builders[resource_bind.memory_type as usize];
                    if hb_cell.is_none() {
                        let mut hb = device.build_dedicated_heap();
                        hb.queue(queue).memory_type(resource_bind.memory_type);
                        *hb_cell = Some(hb);
                    }

                    let hb = hb_cell.as_mut().unwrap();
                    hb.bind(resource_bind.resource);
                }
            }

            // TODO: handle `pass.unbind_resource`.
            //       `DedicatedHeapBuilder` currently does not have
            //       `make_aliasable` yet.
        }

        for maybe_hb in heap_builders {
            if let Some(mut hb) = maybe_hb {
                hb.build()?;
            }

            // Bound resources automatically keep a reference to a heap
        }

        let context = PassInstantiationContext {
            resources: &resources[..],
        };

        let passes: Vec<_> = self
            .passes
            .into_iter()
            .map(|pass| {
                Ok(RunnerPass {
                    pass: AtomicRefCell::new((pass.info.factory)(&context)?),
                    wait_on_passes: pass.wait_on_passes,
                    output: pass.output,
                    update_fence_range: 0..0,
                })
            })
            .collect::<gfx::Result<Vec<_>>>()?;

        Ok(ScheduleRunner {
            queue: queue.clone(),
            passes,
            fences: Vec::new(),
        })
    }
}

impl<'a> ResourceInstantiationContext<'a> {
    pub fn device(&self) -> &'a gfx::DeviceRef {
        self.device
    }

    pub fn queue(&self) -> &'a gfx::CmdQueueRef {
        self.queue
    }
}

impl<'a> PassInstantiationContext<'a> {
    pub fn get_dyn_resource(&self, id: ResourceId) -> &Arc<dyn Resource> {
        &self.resources[id.0]
    }

    /// Borrow a `impl Resource` using a strongly-typed cell identifier.
    pub fn get_resource<T: ResourceInfo>(&self, id: ResourceRef<T>) -> &T::Resource {
        self.get_dyn_resource(id.id())
            .downcast_ref::<T::Resource>()
            .expect("type mismatch")
    }
}

/// Contains and maintains objects and references to device objects required to
/// encode device commands for executing a plan.
#[derive(Debug)]
pub struct ScheduleRunner<C: ?Sized> {
    queue: gfx::CmdQueueRef,
    passes: Vec<RunnerPass<C>>,

    /// `Vec` used to store fences associated with passes. The contents are
    /// only relevant to a single run but the storage persists between runs.
    fences: Vec<gfx::FenceRef>,
}

#[derive(Debug)]
struct RunnerPass<C: ?Sized> {
    pass: AtomicRefCell<Box<dyn Pass<C>>>,
    wait_on_passes: Vec<usize>,

    /// Indicates whether this is an output pass. The completion of all output
    /// passes in a graph indicates that all outputs are ready.
    ///
    /// Note that there might not be a direct correspondence between an output
    /// pass between an output resource.
    output: bool,

    /// A index range into `ScheduleRunner::fences`. This value is initialized when
    /// `ScheduleRunner::run` is called.
    update_fence_range: Range<usize>,
}

impl<C: ?Sized> RunnerPass<C> {
    fn num_update_fences(&self) -> usize {
        self.update_fence_range.len()
    }
}

#[must_use = "`Run` doesn't encode any commands until `encode` is called"]
#[derive(Debug)]
pub struct Run<'a, C: ?Sized> {
    schedule: &'a mut ScheduleRunner<C>,
}

impl<C: ?Sized> ScheduleRunner<C> {
    pub fn num_output_fences(&self) -> usize {
        self.passes
            .iter()
            .filter(|pass| pass.output)
            .map(|pass| pass.pass.borrow().num_update_fences())
            .sum()
    }

    /// Construct a `Run` used to encode commands to evaluate a pass graph
    /// for a single time.
    pub fn run(&mut self) -> gfx::Result<Run<'_, C>> {
        // Allocate fence ranges and create fences.
        let mut i = 0;
        for pass in self.passes.iter_mut() {
            let len = pass.pass.borrow().num_update_fences();
            pass.update_fence_range = i..i + len;
            i += len;
        }

        self.fences.clear();
        self.fences.reserve(i);
        for _ in 0..i {
            self.fences.push(self.queue.new_fence()?);
        }

        Ok(Run { schedule: self })
    }
}

impl<C: ?Sized> Run<'_, C> {
    pub fn num_output_fences(&self) -> usize {
        self.schedule.num_output_fences()
    }

    /// Retrieve an `IteratorMut` providing access to mutable references to
    /// output fences.
    /// The caller may use the output fences automatically created by
    /// [`ScheduleRunner::run`] for a future use, or replace them with custom ones.
    ///
    /// Waiting on all output fences ensures that all outputs of the graph are
    /// ready.
    ///
    /// The number of returned elements is equal to `self.num_output_fences()`.
    pub fn output_fences_mut<'a>(&'a mut self) -> impl IteratorMut<Item = gfx::FenceRef> + 'a {
        (self.schedule.passes)
            .iter_mut()
            .filter(|pass| pass.output)
            .map(|pass| pass.update_fence_range.clone())
            .flatten()
            .gather_mut(&mut self.schedule.fences)
    }

    /// Encode commands into a given command buffer.
    ///
    /// `input_fences` specifies the fences that must be waited for before
    /// executing commands. `input_fences` must include output fences
    /// ([`Run::output_fences_mut`]) of the same `Run` from the previous frame.
    pub fn encode(
        self,
        cmd_buffer: &mut gfx::CmdBufferRef,
        input_fences: &[&gfx::FenceRef],
        context: &C,
    ) -> gfx::Result<()> {
        let schedule = self.schedule;

        let passes = &schedule.passes;
        let fences = &mut schedule.fences;

        let mut wait_fences_storage = Vec::with_capacity(
            passes
                .iter()
                .map(|pass| {
                    pass.wait_on_passes
                        .iter()
                        .map(|&i| passes[i].num_update_fences())
                        .sum()
                })
                .max()
                .unwrap_or(0),
        );
        let mut update_fences_storage =
            Vec::with_capacity(passes.iter().map(|pass| pass.num_update_fences()).sum());

        for pass in passes {
            let wait_fences = if pass.wait_on_passes.len() == 0 {
                input_fences
            } else {
                wait_fences_storage.clear();
                wait_fences_storage.extend(
                    pass.wait_on_passes
                        .iter()
                        .map(|&i| &fences[passes[i].update_fence_range.clone()])
                        .flatten(),
                );
                &wait_fences_storage[..]
            };

            // `&[T]` to `&[&T]` conversion
            update_fences_storage.clear();
            update_fences_storage.extend(&fences[pass.update_fence_range.clone()]);

            let update_fences = &update_fences_storage[..];

            pass.pass
                .borrow_mut()
                .encode(cmd_buffer, wait_fences, update_fences, context)?;
        }

        Ok(())
    }
}
