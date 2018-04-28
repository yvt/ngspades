//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::Vector2;
use ngscom::{hresults, to_hresult, ComPtr, HResult, IAny, IUnknown};
use owning_ref::OwningRefMut;
use rgb::RGBA;
use stickylock::{StickyMutex, StickyMutexGuard};

use canvas::painter::Painter;
use ngsbase::{IPainter, IPainterTrait};

com_impl! {
    /// A COM wrapper for `ngspf::canvas::Painter`.
    class ComPainter {
        i_painter: IPainter;
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

impl IPainterTrait for ComPainter {
    fn end(&self) -> HResult {
        to_hresult(|| {
            let mut painter_cell = self.data.painter.lock();
            *painter_cell = None;
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
}
