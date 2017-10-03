//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::Vector3;
use cgmath::prelude::*;

/// Value that changes dynamically in a piecewise linear fashion.
#[derive(Debug, Clone, Copy)]
pub struct DynamicValue {
    pub current: f64,
    pub change_rate: f64,
    pub goal: f64,
}

impl DynamicValue {
    pub fn new(x: f64) -> Self {
        DynamicValue {
            current: x,
            change_rate: 0f64,
            goal: x,
        }
    }

    pub fn update(&mut self) {
        if self.change_rate > 0f64 {
            self.current = (self.current + self.change_rate).min(self.goal);
            if self.current >= self.goal {
                self.change_rate = 0f64;
            }
        } else if self.change_rate < 0f64 {
            self.current = (self.current + self.change_rate).max(self.goal);
            if self.current <= self.goal {
                self.change_rate = 0f64;
            }
        }
    }

    pub fn update_multi(&mut self, duration: f64) {
        if self.change_rate > 0f64 {
            self.current = (self.current + self.change_rate * duration).min(self.goal);
            if self.current >= self.goal {
                self.change_rate = 0f64;
            }
        } else if self.change_rate < 0f64 {
            self.current = (self.current + self.change_rate * duration).max(self.goal);
            if self.current <= self.goal {
                self.change_rate = 0f64;
            }
        }
    }

    pub fn next_cusp_time(&self, within_duration: usize) -> usize {
        let duration = within_duration as f64;
        if (self.change_rate > 0f64 && self.current + (self.change_rate * duration) > self.goal) ||
            (self.change_rate < 0f64 && self.current + (self.change_rate * duration) < self.goal)
        {
            ((self.goal - self.current) / self.change_rate).ceil() as usize
        } else {
            within_duration
        }
    }

    pub fn get(&self) -> f64 {
        self.current
    }

    pub fn set(&mut self, new: f64) {
        assert!(new.is_finite());
        self.current = new;
        self.change_rate = 0f64;
        self.goal = new;
    }

    pub fn is_stationary(&self) -> bool {
        self.change_rate != 0.0
    }

    pub fn set_slow(&mut self, new: f64, duration: f64) {
        assert!(new.is_finite());
        assert!(duration.is_finite());
        if duration <= 0f64 {
            self.set(new);
        } else {
            self.goal = new;
            self.change_rate = (new - self.current) / duration;
        }
    }
}

/// 3D unit vector version of `DynamicValue`.
///
/// FIXME: This can be generalized to non-3D vectors
#[derive(Debug, Clone, Copy)]
pub struct DynamicSlerpVector3 {
    pub current: Vector3<f64>,
    pub change_duration: f64,
    pub goal: Vector3<f64>,
}

impl DynamicSlerpVector3 {
    pub fn new(mut x: Vector3<f64>) -> Self {
        x = x.normalize();
        DynamicSlerpVector3 {
            current: x,
            change_duration: 0.0,
            goal: x,
        }
    }

    pub fn update(&mut self) {
        self.update_multi(1.0);
    }

    pub fn update_multi(&mut self, duration: f64) {
        if self.change_duration > 0.0 {
            let change = self.change_duration.min(duration);

            if change >= self.change_duration {
                self.current = self.goal;
                self.change_duration = 0.0;
            } else {
                let r = change / self.change_duration;
                let cos = self.current.dot(self.goal).min(1.0).max(-1.0);
                let (coef1, coef2) = if cos > 1.0 - 1.0e-6 {
                    // linear approximation (numerically more stable for small
                    // arcs)
                    (1.0 - r, r)
                } else {
                    let arc_angle = cos.acos();
                    let isin = 1.0 / (1.0 - cos * cos).sqrt();
                    let coef1 = (arc_angle * (1.0 - r)).sin() * isin;
                    let coef2 = (arc_angle * r).sin() * isin;
                    (coef1, coef2)
                };
                self.current = (self.current * coef1 + self.goal * coef2).normalize();
                self.change_duration -= change;
            }
        }
    }

    pub fn next_cusp_time(&self, within_duration: usize) -> usize {
        if self.change_duration > 0.0 {
            let duration = within_duration as f64;
            duration.min(self.change_duration).ceil() as usize
        } else {
            within_duration
        }
    }

    pub fn get(&self) -> Vector3<f64> {
        self.current
    }

    pub fn set(&mut self, mut new: Vector3<f64>) {
        new = new.normalize();
        assert!(new.x.is_finite());
        assert!(new.y.is_finite());
        assert!(new.z.is_finite());
        self.current = new;
        self.goal = new;
        self.change_duration = 0.0;
    }

    pub fn set_slow(&mut self, mut new: Vector3<f64>, duration: f64) {
        new = new.normalize();
        assert!(new.x.is_finite());
        assert!(new.y.is_finite());
        assert!(new.z.is_finite());
        assert!(duration.is_finite());
        if duration <= 0f64 {
            self.set(new);
        } else {
            self.goal = new;
            self.change_duration = duration;
        }
    }
}
