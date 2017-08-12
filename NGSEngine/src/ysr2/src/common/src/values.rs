//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

/// Value that changes dynamically in a piecewise linear fashion.
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
