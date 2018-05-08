//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::Vector2;
use ngscom::{hresults, to_hresult, ComPtr, HResult, IAny, IUnknown, UnownedComPtr};
use owning_ref::OwningRefMut;
use rgb::RGBA;
use stickylock::{StickyMutex, StickyMutexGuard};

use canvas::painter::Painter;
use hresults::E_PF_THREAD;
use ngsbase::{INgsPFPainter, INgsPFPainterTrait, INgsPFTextLayout};

use text::ComTextLayout;

com_impl! {
    /// A COM wrapper for `ngspf::canvas::Painter`.
    class ComPainter {
        ingspf_painter: INgsPFPainter;
        iany: IAny;
        @data: PainterData;
    }
}

struct PainterData {
    // The painter has to be boxed because `com_impl!` doesn't support generics
    painter: StickyMutex<Option<Box<AsMut<Painter + Send + Sync> + Send + Sync>>>,
}

impl ComPainter {
    /// Construct a `ComPainter` from a given `AsMut<Painter>` and return it as
    /// `IUnknown`.
    pub fn new<T: AsMut<Painter + Send + Sync> + Send + Sync + 'static>(
        painter: T,
    ) -> ComPtr<IUnknown> {
        Self::alloc(PainterData {
            painter: StickyMutex::new(Some(Box::new(painter))),
        })
    }

    /// Acqure a lock on the contained painter. Fails iff `finish` already has
    /// been called on `self`.
    fn lock_painter(
        &self,
    ) -> Option<
        OwningRefMut<
            StickyMutexGuard<Option<Box<AsMut<Painter + Send + Sync> + Send + Sync>>>,
            Painter + Send + Sync,
        >,
    > {
        Some(
            OwningRefMut::new(self.data.painter.lock())
                .try_map_mut(|x| x.as_mut().ok_or(()))
                .ok()?
                .map_mut(|x| (**x).as_mut()),
        )
    }
}

impl INgsPFPainterTrait for ComPainter {
    fn finish(&self) -> HResult {
        to_hresult(|| {
            let mut painter_cell = self.data.painter.lock();
            *painter_cell = None;
            Ok(())
        })
    }

    fn lock(&self) -> HResult {
        to_hresult(|| {
            self.data.painter.stick();
            Ok(())
        })
    }

    fn unlock(&self) -> HResult {
        to_hresult(|| {
            self.data.painter.unstick().map_err(|_| E_PF_THREAD)?;
            Ok(())
        })
    }

    fn translate(&self, offset: Vector2<f32>) -> HResult {
        to_hresult(|| {
            let mut painter = self.lock_painter().ok_or(hresults::E_UNEXPECTED)?;
            painter.translate(offset.cast());
            Ok(())
        })
    }

    fn non_uniform_scale(&self, x: f32, y: f32) -> HResult {
        to_hresult(|| {
            let mut painter = self.lock_painter().ok_or(hresults::E_UNEXPECTED)?;
            painter.nonuniform_scale(x as f64, y as f64);
            Ok(())
        })
    }

    fn restore(&self) -> HResult {
        to_hresult(|| {
            let mut painter = self.lock_painter().ok_or(hresults::E_UNEXPECTED)?;
            painter.restore();
            Ok(())
        })
    }

    fn save(&self) -> HResult {
        to_hresult(|| {
            let mut painter = self.lock_painter().ok_or(hresults::E_UNEXPECTED)?;
            painter.save();
            Ok(())
        })
    }

    fn set_fill_color(&self, color: RGBA<f32>) -> HResult {
        to_hresult(|| {
            let mut painter = self.lock_painter().ok_or(hresults::E_UNEXPECTED)?;
            painter.set_fill_color(color);
            Ok(())
        })
    }

    fn fill_text_layout(&self, text_layout: UnownedComPtr<INgsPFTextLayout>) -> HResult {
        to_hresult(|| {
            let layout: ComPtr<IAny> = (&*text_layout).into();
            let layout = layout.non_null().ok_or(hresults::E_UNEXPECTED)?;
            let layout: &ComTextLayout = layout.downcast_ref().ok_or(hresults::E_UNEXPECTED)?;

            let mut painter = self.lock_painter().ok_or(hresults::E_UNEXPECTED)?;

            layout.with_layout_config(|text_layout, font_config| {
                painter.fill_text_layout(text_layout, font_config);
            });

            Ok(())
        })
    }
}
