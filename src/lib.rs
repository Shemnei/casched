//! TODO
//! - Have lifetimed scheduler (e.g. non static functions)
//! - Measure jitter / long running functions to make predictions?
//! - Counted scheduler (e.g. run 5 times then remove)

use std::{
    collections::BinaryHeap,
    matches,
    ops::Add,
    time::{Duration, Instant},
};

/// An [`std::time::Instant`] wrapper with the main purpose of reversing the
/// ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stbi(Instant);

impl Stbi {
    pub fn now() -> Self {
        Self(Instant::now())
    }

    pub fn since(&self, earlier: Self) -> Duration {
        self.0.duration_since(earlier.0)
    }
}

impl PartialOrd for Stbi {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (other.0).partial_cmp(&self.0)
    }
}

impl Ord for Stbi {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (other.0).cmp(&self.0)
    }
}

impl Add<Duration> for Stbi {
    type Output = Stbi;

    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0.add(rhs))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schedule {
    Once(Option<Duration>),
    Every(Duration),
}

impl Schedule {
    pub fn is_once(&self) -> bool {
        matches!(self, Self::Once(_))
    }

    pub fn as_duration(&self) -> &Duration {
        match self {
            Schedule::Once(duration) => duration.as_ref().unwrap_or(&Duration::ZERO),
            Schedule::Every(d) => d,
        }
    }

    pub fn with<F>(self, f: F) -> Task
    where
        F: 'static + FnMut(),
    {
        Task {
            schedule: self,
            f: Box::new(f),
        }
    }

    pub fn with_boxed(self, f: TaskFunction) -> Task {
        Task { schedule: self, f }
    }
}

pub type TaskFunction = Box<dyn FnMut() + 'static>;

pub struct Task {
    schedule: Schedule,
    f: TaskFunction,
}

impl Task {
    pub fn once<F>(f: F) -> Self
    where
        F: 'static + FnMut(),
    {
        Schedule::Once(None).with(f)
    }

    pub fn once_boxed(f: TaskFunction) -> Self {
        Schedule::Once(None).with_boxed(f)
    }

    pub fn offset<F>(duration: Duration, f: F) -> Self
    where
        F: 'static + FnMut(),
    {
        Schedule::Once(Some(duration)).with(f)
    }

    pub fn offset_boxed<F>(duration: Duration, f: TaskFunction) -> Self {
        Schedule::Once(Some(duration)).with_boxed(f)
    }

    pub fn every<F>(duration: Duration, f: F) -> Self
    where
        F: 'static + FnMut(),
    {
        Schedule::Every(duration).with(f)
    }

    pub fn every_boxed(duration: Duration, f: TaskFunction) -> Self {
        Schedule::Every(duration).with_boxed(f)
    }
}

pub struct ScheduledTask {
    at: Stbi,
    task: Task,
}

impl PartialEq for ScheduledTask {
    fn eq(&self, other: &Self) -> bool {
        self.at == other.at
    }
}

impl Eq for ScheduledTask {}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.at.partial_cmp(&other.at)
    }
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.at.cmp(&other.at)
    }
}

pub struct Scheduler {
    schedule: BinaryHeap<ScheduledTask>,
}

impl Scheduler {
    pub fn with_tasks(tasks: Vec<Task>) -> Self {
        let mut schedule = BinaryHeap::new();

        let now = Stbi::now();

        for task in tasks {
            let at = now + *task.schedule.as_duration();
            let task = ScheduledTask { at, task };
            schedule.push(task)
        }

        Self { schedule }
    }

    pub fn run(mut self) {
        loop {
            let now = Stbi::now();

            let Some(top) = self.schedule.peek() else {
                return;
            };

            let diff = top.at.since(now);

            if diff.is_zero() {
                // We are past the `at` timestamp
                let mut task = self.schedule.pop().expect("Peek returned value");

                (task.task.f)();

                // Push next execution
                if !task.task.schedule.is_once() {
                    task.at = now + *task.task.schedule.as_duration();
                    self.schedule.push(task);
                }
            } else {
                std::thread::sleep(diff);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::println;

    use super::*;

    #[test]
    fn simple_schedule() {
        let tasks = vec![
            Schedule::Once(Some(Duration::from_secs(5))).with(|| {
                println!("Delayed hello world");
            }),
            Schedule::Once(None).with(|| {
                println!("Instant hello world");
            }),
            Schedule::Every(Duration::from_millis(125)).with(|| {
                println!("I am annoying");
            }),
            Schedule::Every(Duration::from_millis(125)).with(|| {
                println!("I am annoying too");
            }),
            Schedule::Every(Duration::from_millis(66)).with(|| {
                println!("I am annoying thrice");
            }),
        ];

        let scheduler = Scheduler::with_tasks(tasks);

        scheduler.run();
    }
}
