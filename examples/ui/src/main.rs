use std::sync::{Arc, RwLock};
use std::thread;

use anyhow::Error;
use fadetop::priority::SpiedRecordQueueMap;
use fadetop::{app::FadeTopApp, priority::SamplerOps};
use py_spy::stack_trace::LocalVariable;
use py_spy::{Frame, StackTrace};

#[derive(Clone, Debug, Default)]
struct MockSampler {}

impl SamplerOps for MockSampler {
    fn push_to_queue(self, queue: Arc<RwLock<SpiedRecordQueueMap>>) -> Result<(), Error> {
        loop {
            let frame_template = Frame {
                name: "level0".to_string(),
                filename: "test.py".to_string(),
                line: 1,
                module: Some("test".to_string()),
                short_filename: Some("test.py".to_string()),
                locals: Some(vec![LocalVariable {
                    name: "x".to_string(),
                    addr: 10,
                    arg: true,
                    repr: Some("data, verryyyyyy looonnnnnnng data".to_string()),
                }]),
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

            for _ in 0..10 {
                thread::sleep(std::time::Duration::from_millis(10));
                queue.write().unwrap().increment(&trace);
            }

            for _ in 0..10 {
                thread::sleep(std::time::Duration::from_millis(10));
                queue.write().unwrap().increment(&trace);
            }
            for _ in 0..10 {
                thread::sleep(std::time::Duration::from_millis(10));

                let total_events: usize = queue
                    .read()
                    .unwrap()
                    .iter()
                    .map(|(_, x)| x.finished_events.len())
                    .sum();
                queue.write().unwrap().increment(&StackTrace {
                    frames: vec![
                        Frame {
                            name: "level3".to_string(),
                            locals: Some(vec![LocalVariable {
                                name: "x".to_string(),
                                addr: 10,
                                arg: true,
                                repr: Some(format!("{:?}", total_events)),
                            }]),
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
            }

            thread::sleep(std::time::Duration::from_millis(10));
            queue.write().unwrap().increment(&StackTrace {
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

            for _ in 0..10 {
                thread::sleep(std::time::Duration::from_millis(10));
                queue.write().unwrap().increment(&StackTrace {
                    frames: vec![Frame {
                        name: "level2_different".to_string(),
                        ..frame_template.clone()
                    }],
                    thread_id: 2,
                    ..trace.clone()
                });
            }
        }
    }
}

fn main() -> Result<(), Error> {
    let terminal = ratatui::init();
    let app = FadeTopApp::new(fadetop::config::AppConfig::from_configs()?);

    let result = app.run(terminal, MockSampler {});
    ratatui::restore();
    result
}
