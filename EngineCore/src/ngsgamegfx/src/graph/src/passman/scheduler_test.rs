//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use crate::passman;
use zangfx::base as gfx;

#[derive(Debug)]
struct MyResourceInfo(usize);

impl passman::ResourceInfo for MyResourceInfo {
    type Resource = passman::ImageResource;

    fn build(
        &self,
        _context: &passman::ResourceInstantiationContext<'_>,
    ) -> gfx::Result<Box<Self::Resource>> {
        unreachable!()
    }
}

#[test]
fn test() {
    let mut builder = passman::ScheduleBuilder::<()>::new();

    let res0 = builder.define_resource(MyResourceInfo(1));
    let res1 = builder.define_resource(MyResourceInfo(2));
    let res2 = builder.define_resource(MyResourceInfo(3));

    assert_eq!(builder.get_resource_info_mut(res0).0, 1);
    assert_eq!(builder.get_resource_info_mut(res1).0, 2);
    assert_eq!(builder.get_resource_info_mut(res2).0, 3);

    builder.define_pass(passman::PassInfo {
        resource_uses: vec![res0.use_as_producer()],
        factory: Box::new(|_| unreachable!()),
    });

    builder.define_pass(passman::PassInfo {
        resource_uses: vec![res0.use_as_consumer(), res1.use_as_producer()],
        factory: Box::new(|_| unreachable!()),
    });

    builder.define_pass(passman::PassInfo {
        resource_uses: vec![
            res0.use_as_consumer(),
            res1.use_as_consumer(),
            res2.use_as_producer(),
        ],
        factory: Box::new(|_| unreachable!()),
    });

    let schedule = builder.schedule(&[&res2]);

    println!("{:#?}", schedule);

    // This graph has a unique solution.
    assert_eq!(schedule.passes[0].wait_on_passes, vec![]);
    assert_eq!(schedule.passes[1].wait_on_passes, vec![0]);
    assert_eq!(schedule.passes[2].wait_on_passes, vec![0, 1]);

    assert_eq!(schedule.passes[0].bind_resources, vec![0]);
    assert_eq!(schedule.passes[0].unbind_resources, vec![]);

    assert_eq!(schedule.passes[1].bind_resources, vec![1]);
    assert_eq!(schedule.passes[1].unbind_resources, vec![]);

    assert_eq!(schedule.passes[2].bind_resources, vec![2]);
    assert_eq!(schedule.passes[2].unbind_resources, vec![0, 1, 2]);
}

#[test]
#[should_panic]
fn panic_on_cyclic_dependency() {
    let mut builder = passman::ScheduleBuilder::<()>::new();

    let res0 = builder.define_resource(MyResourceInfo(1));
    let res1 = builder.define_resource(MyResourceInfo(2));

    builder.define_pass(passman::PassInfo {
        resource_uses: vec![res0.use_as_consumer(), res1.use_as_producer()],
        factory: Box::new(|_| unreachable!()),
    });

    builder.define_pass(passman::PassInfo {
        resource_uses: vec![res1.use_as_consumer(), res0.use_as_producer()],
        factory: Box::new(|_| unreachable!()),
    });

    let schedule = builder.schedule(&[&res1]);

    println!("{:#?}", schedule);
}
