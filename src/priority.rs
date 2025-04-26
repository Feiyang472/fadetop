use anyhow::Error;
use py_spy::sampler;
use py_spy::stack_trace::Frame;
use py_spy::stack_trace::LocalVariable;
use py_spy::stack_trace::StackTrace;
use remoteprocess::{Pid, Tid};
use serde::Deserialize;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::collections::hash_map::Iter;
use std::collections::hash_map::Keys;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;
use std::time::Instant;

use crate::ser::parse_duration;

#[derive(Debug, Clone, Default)]
pub struct ThreadInfo {
    pub name: Option<String>,
    pub pid: Pid,
    pub tid: Tid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameKey {
    filename: String,
    pub name: String,
}

impl FrameKey {
    fn should_merge(&self, b: &Frame) -> bool {
        self.name == b.name && self.filename == b.filename
    }

    pub fn fqn(&self) -> String {
        format!("{}::{}", self.filename, self.name)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinishedRecord {
    pub frame_key: FrameKey,
    pub start: Instant,
    pub end: Instant,
    pub depth: usize,
    forget_time: ForgetTime,
}

impl Ord for FinishedRecord {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Reverse(self.forget_time).cmp(&Reverse(other.forget_time))
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
    locals: Option<Vec<LocalVariable>>,
}

impl UnfinishedRecord {
    pub fn locals(&self) -> Option<&Vec<LocalVariable>> {
        self.locals.as_ref()
    }
}

#[derive(Clone, Debug)]
pub struct SpiedRecordQueue {
    pub unfinished_events: Vec<UnfinishedRecord>,
    pub finished_events: BinaryHeap<FinishedRecord>,
    pub start_ts: Instant,
    pub last_update: Instant,
    pub thread_info: ThreadInfo,
}

impl SpiedRecordQueue {
    fn new(thread_info: ThreadInfo) -> Self {
        SpiedRecordQueue {
            finished_events: BinaryHeap::new(),
            unfinished_events: vec![],
            start_ts: Instant::now(),
            last_update: Instant::now(),
            thread_info,
        }
    }

    pub fn thread_name<'a>(&'a self) -> &'a Option<String> {
        &self.thread_info.name
    }
}

fn event(
    frame_key: FrameKey,
    start: Instant,
    end: Instant,
    depth: usize,
    forget_time: ForgetTime,
) -> FinishedRecord {
    FinishedRecord {
        frame_key,
        start,
        end,
        depth,
        forget_time,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ForgetTime {
    When(Instant),
    Never,
}

impl Ord for ForgetTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (ForgetTime::When(a), ForgetTime::When(b)) => a.cmp(b),
            (ForgetTime::Never, ForgetTime::Never) => std::cmp::Ordering::Equal,
            (ForgetTime::When(_), ForgetTime::Never) => std::cmp::Ordering::Less,
            (ForgetTime::Never, ForgetTime::When(_)) => std::cmp::Ordering::Greater,
        }
    }
}

impl PartialOrd for ForgetTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ForgetRules {
    LastedLessThan(#[serde(deserialize_with = "parse_duration")] Duration),
    RectLinear {
        #[serde(deserialize_with = "parse_duration")]
        at_least: Duration,
        ratio: f32,
    },
}

impl ForgetRules {
    fn pop_time(&self, start: Instant, end: Instant) -> ForgetTime {
        match *self {
            Self::LastedLessThan(period) => {
                if period < end - start {
                    ForgetTime::When(end)
                } else {
                    ForgetTime::Never
                }
            }
            Self::RectLinear { at_least, ratio } => {
                ForgetTime::When(end + at_least + (end - start).mul_f32(ratio))
            }
        }
    }
}

fn forget_time(rules: &Vec<ForgetRules>, start: Instant, end: Instant) -> ForgetTime {
    rules
        .iter()
        .map(|rule| rule.pop_time(start, end))
        .min()
        .unwrap_or(ForgetTime::Never)
}

#[derive(Debug, Default)]
pub struct SpiedRecordQueueMap {
    map: HashMap<Tid, SpiedRecordQueue>,
    rules: Vec<ForgetRules>,
}

impl SpiedRecordQueueMap {
    pub fn keys(&self) -> Keys<'_, Tid, SpiedRecordQueue> {
        self.map.keys()
    }
    pub fn iter(&self) -> Iter<'_, Tid, SpiedRecordQueue> {
        self.map.iter()
    }
    pub fn get(&self, k: &Tid) -> Option<&SpiedRecordQueue> {
        self.map.get(k)
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn contains_key(&self, k: &Tid) -> bool {
        self.map.contains_key(k)
    }

    pub fn with_rules(&mut self, rules: Vec<ForgetRules>) {
        self.rules = rules;
    }

    pub fn increment(&mut self, trace: &StackTrace) {
        let now = Instant::now();

        self.map.retain(|_, queue| {
            while let Some(top) = queue.finished_events.peek() {
                match top.forget_time {
                    ForgetTime::Never => return true,
                    ForgetTime::When(time) => {
                        if time > now {
                            return true;
                        } else {
                            queue.finished_events.pop().unwrap();
                        }
                    }
                }
            }
            !queue.unfinished_events.is_empty()
                && match forget_time(&self.rules, queue.start_ts, queue.last_update) {
                    ForgetTime::When(when) => when > now,
                    ForgetTime::Never => true,
                }
        });

        let mut queue = self
            .map
            .remove(&(trace.thread_id as Tid))
            .unwrap_or_else(|| {
                SpiedRecordQueue::new(ThreadInfo {
                    name: trace.thread_name.clone(),
                    pid: trace.pid,
                    tid: trace.thread_id as Tid,
                })
            });

        let mut prev_frames = queue.unfinished_events;

        let mut new_idx = 0;

        for (prev, new) in prev_frames.iter_mut().zip(trace.frames.iter().rev()) {
            if prev.frame_key.should_merge(new) {
                prev.locals = new.locals.clone();
                new_idx += 1;
                continue;
            } else {
                break;
            }
        }

        for depth in (new_idx..prev_frames.len()).rev() {
            let unfinished = prev_frames.pop().unwrap(); // safe
            queue.finished_events.push(event(
                unfinished.frame_key,
                unfinished.start,
                now,
                depth,
                forget_time(&self.rules, unfinished.start, now),
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
                },
                locals: frame.locals.clone(),
            });
        }

        // Save this stack trace for the next iteration.
        queue.unfinished_events = prev_frames;
        queue.last_update = now;

        self.map.insert(trace.thread_id as Tid, queue);
    }
}

pub trait SamplerOps: Send + 'static {
    fn push_to_queue(self, record_queue_map: Arc<RwLock<SpiedRecordQueueMap>>)
    -> Result<(), Error>;
}

impl SamplerOps for sampler::Sampler {
    fn push_to_queue(
        self,
        record_queue_map: Arc<RwLock<SpiedRecordQueueMap>>,
    ) -> Result<(), Error> {
        for sample in self {
            for trace in sample.traces.iter() {
                record_queue_map
                    .write()
                    .map_err(|_| std::sync::PoisonError::new(trace.thread_id))?
                    .increment(trace);
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
    fn test_compare_record() {
        let now = Instant::now();
        let rec1 = FinishedRecord {
            frame_key: FrameKey {
                filename: "".to_string(),
                name: "".to_string(),
            },
            start: now,
            end: now,
            depth: 0,
            forget_time: ForgetTime::When(now),
        };

        let rec2 = FinishedRecord {
            forget_time: ForgetTime::When(now + Duration::from_secs(1)),
            ..rec1.clone()
        };
        assert!(rec1 > rec2);

        let rec3 = FinishedRecord {
            forget_time: ForgetTime::Never,
            ..rec1.clone()
        };
        assert!(rec1 > rec3, "rec1 should be popped before rec3");
    }

    #[test]
    fn test_inserting_frames() {
        let mut queues = SpiedRecordQueueMap::default();
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

        queues.increment(&trace);
        assert_eq!(queues.map[&1].unfinished_events.len(), 2);
        assert_eq!(queues.map[&1].finished_events.len(), 0);

        queues.increment(&trace);
        assert_eq!(queues.map[&1].unfinished_events.len(), 2);
        assert_eq!(queues.map[&1].finished_events.len(), 0);

        queues.increment(&StackTrace {
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
            queues.map[&1]
                .unfinished_events
                .iter()
                .map(|event| event.frame_key.name.clone())
                .collect::<Vec<String>>(),
            vec!["level0", "level1_different", "level2", "level3"]
        );
        assert_eq!(
            queues.map[&1]
                .finished_events
                .iter()
                .map(|event| event.frame_key.name.clone())
                .collect::<Vec<String>>(),
            vec!["level1",]
        );

        queues.increment(&StackTrace {
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
            queues.map[&1]
                .unfinished_events
                .iter()
                .map(|event| event.frame_key.name.clone())
                .collect::<Vec<String>>(),
            vec!["level0", "level1_different", "level2_different"]
        );
        assert_eq!(
            queues.map[&1]
                .finished_events
                .iter()
                .map(|event| event.frame_key.name.clone())
                .collect::<Vec<String>>(),
            vec!["level1", "level3", "level2"]
        );

        queues.increment(&StackTrace {
            frames: vec![Frame {
                name: "level2_different".to_string(),
                ..frame_template.clone()
            }],
            thread_id: 2,
            ..trace.clone()
        });

        assert_eq!(queues.map[&1].finished_events.len(), 3);
        assert_eq!(queues.map[&1].unfinished_events.len(), 3);
        assert_eq!(queues.map[&2].unfinished_events.len(), 1);
    }
}
