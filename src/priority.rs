use anyhow::Error;
use py_spy::sampler;
use py_spy::stack_trace::Frame;
use py_spy::stack_trace::StackTrace;
use remoteprocess::Tid;
use std::cmp::min;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameKey {
    pub filename: String,
    pub name: String,
    pub pid: i32,
    pub tid: Tid,
}

impl FrameKey {
    fn should_merge(&self, b: &Frame) -> bool {
        self.name == b.name && self.filename == b.filename
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinishedRecord {
    pub frame_key: FrameKey,
    pub start: Instant,
    pub end: Instant,
    pub depth: usize,
}

impl Ord for FinishedRecord {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.end.cmp(&other.end).reverse()
    }
}

impl PartialOrd for FinishedRecord {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
pub struct UnfinishedRecord {
    pub frame_key: FrameKey,
    pub start: Instant,
}

#[derive(Clone, Debug)]
pub struct ForgettingQueue {
    pub finished_events: BinaryHeap<FinishedRecord>,
    pub start_ts: Instant,
    pub unfinished_events: HashMap<Tid, Vec<UnfinishedRecord>>,
}

impl ForgettingQueue {
    pub fn new() -> ForgettingQueue {
        ForgettingQueue {
            finished_events: BinaryHeap::new(),
            start_ts: Instant::now(),
            unfinished_events: HashMap::new(),
        }
    }

    fn event(
        &self,
        trace: &StackTrace,
        frame: &FrameKey,
        start: Instant,
        end: Instant,
        depth: usize,
    ) -> FinishedRecord {
        FinishedRecord {
            frame_key: FrameKey {
                tid: trace.thread_id as Tid,
                pid: trace.pid,
                name: frame.name.clone(),
                filename: frame.filename.clone(),
            },
            start,
            end,
            depth,
        }
    }

    pub fn increment(&mut self, trace: &StackTrace) {
        let now = Instant::now();

        let mut prev_frames = self
            .unfinished_events
            .remove(&(trace.thread_id as Tid))
            .unwrap_or_default();

        let new_idx = prev_frames
            .iter()
            .zip(trace.frames.iter().rev())
            .position(|(prev, new)| !prev.frame_key.should_merge(new))
            .unwrap_or(min(prev_frames.len(), trace.frames.len()));

        for depth in (new_idx..prev_frames.len()).rev() {
            let unfinished = prev_frames.pop().unwrap();
            self.finished_events.push(self.event(
                trace,
                &unfinished.frame_key,
                unfinished.start,
                now,
                depth,
            ));
        }

        for frame in trace.frames[..trace.frames.len().saturating_sub(new_idx)]
            .iter()
            .rev()
        {
            prev_frames.push(UnfinishedRecord {
                start: now,
                frame_key: FrameKey {
                    filename: frame.filename.clone(),
                    name: frame.name.clone(),
                    pid: trace.pid,
                    tid: trace.thread_id as Tid,
                },
            });
        }

        // Save this stack trace for the next iteration.
        self.unfinished_events
            .insert(trace.thread_id as Tid, prev_frames);
    }
}

pub trait SamplerOps: Send + 'static {
    fn push_to_queue(self, forgetting_queue: Arc<RwLock<ForgettingQueue>>) -> Result<(), Error>;
}

impl SamplerOps for sampler::Sampler {
    fn push_to_queue(self, forgetting_queue: Arc<RwLock<ForgettingQueue>>) -> Result<(), Error> {
        for mut sample in self {
            for trace in sample.traces.iter_mut() {
                let threadid = trace.format_threadid();
                let thread_fmt = if let Some(thread_name) = &trace.thread_name {
                    format!("thread ({}): {}", threadid, thread_name)
                } else {
                    format!("thread ({})", threadid)
                };
                trace.frames.push(Frame {
                    name: thread_fmt,
                    filename: String::from(""),
                    module: None,
                    short_filename: None,
                    line: 0,
                    locals: None,
                    is_entry: true,
                });

                if let Some(process_info) = trace.process_info.as_ref() {
                    trace.frames.push(process_info.to_frame());
                    let mut parent = process_info.parent.as_ref();
                    while parent.is_some() {
                        if let Some(process_info) = parent {
                            trace.frames.push(process_info.to_frame());
                            parent = process_info.parent.as_ref();
                        }
                    }
                }

                forgetting_queue.write().unwrap().increment(trace);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use py_spy::stack_trace::StackTrace;

    #[test]
    fn test_inserting_frames() {
        let mut queue = ForgettingQueue::new();
        let frame_template = Frame {
            name: "level0".to_string(),
            filename: "test.py".to_string(),
            line: 1,
            module: Some("test".to_string()),
            short_filename: Some("test.py".to_string()),
            locals: None,
            is_entry: false,
        };

        let trace = StackTrace {
            thread_id: 1,
            pid: 1,
            frames: vec![
                Frame {
                    name: "level1".to_string(),
                    ..frame_template.clone()
                },
                frame_template.clone(),
            ],
            thread_name: None,
            os_thread_id: None,
            active: true,
            owns_gil: false,
            process_info: None,
        };

        queue.increment(&trace);
        assert_eq!(queue.unfinished_events[&1].len(), 2);
        assert_eq!(queue.finished_events.len(), 0);

        queue.increment(&trace);
        assert_eq!(queue.unfinished_events[&1].len(), 2);
        assert_eq!(queue.finished_events.len(), 0);

        queue.increment(&StackTrace {
            frames: vec![
                Frame {
                    name: "level3".to_string(),
                    ..frame_template.clone()
                },
                Frame {
                    name: "level2".to_string(),
                    ..frame_template.clone()
                },
                Frame {
                    name: "level1_different".to_string(),
                    ..frame_template.clone()
                },
                trace.frames[1].clone(),
            ],
            ..trace.clone()
        });
        assert_eq!(
            queue.unfinished_events[&1]
                .iter()
                .map(|event| event.frame_key.name.clone())
                .collect::<Vec<String>>(),
            vec!["level0", "level1_different", "level2", "level3"]
        );
        assert_eq!(
            queue
                .finished_events
                .iter()
                .map(|event| event.frame_key.name.clone())
                .collect::<Vec<String>>(),
            vec!["level1",]
        );

        queue.increment(&StackTrace {
            frames: vec![
                Frame {
                    name: "level2_different".to_string(),
                    ..frame_template.clone()
                },
                Frame {
                    name: "level1_different".to_string(),
                    ..frame_template.clone()
                },
                trace.frames[1].clone(),
            ],
            ..trace.clone()
        });
        assert_eq!(
            queue.unfinished_events[&1]
                .iter()
                .map(|event| event.frame_key.name.clone())
                .collect::<Vec<String>>(),
            vec!["level0", "level1_different", "level2_different"]
        );
        assert_eq!(
            queue
                .finished_events
                .iter()
                .map(|event| event.frame_key.name.clone())
                .collect::<Vec<String>>(),
            vec!["level1", "level3", "level2"]
        );

        queue.increment(&StackTrace {
            frames: vec![Frame {
                name: "level2_different".to_string(),
                ..frame_template.clone()
            }],
            thread_id: 2,
            ..trace.clone()
        });

        assert_eq!(queue.finished_events.len(), 3);
        assert_eq!(queue.unfinished_events[&1].len(), 3);
        assert_eq!(queue.unfinished_events[&2].len(), 1);
    }
}
