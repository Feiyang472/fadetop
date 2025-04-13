use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;

use anyhow::Error;
use fadetop::priority::ForgettingQueueMap;
use fadetop::priority::ForgettingQueueMapOps;
use fadetop::{
    app::{FadeTopApp, SamplerFactory},
    priority::SamplerOps,
};
use py_spy::{Frame, StackTrace};

#[derive(Clone, Debug, Default)]
struct MockSampler {}

impl SamplerOps for MockSampler {
    fn push_to_queue(self, queue: Arc<RwLock<ForgettingQueueMap>>) -> Result<(), Error> {
        loop {
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
                queue.write().unwrap().increment(&StackTrace {
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

impl SamplerFactory for MockSampler {
    type Sampler = MockSampler;

    fn create_sampler(&self) -> Result<Self::Sampler, Error> {
        Ok(MockSampler {})
    }
}

fn main() -> Result<(), Error> {
    let terminal = ratatui::init();
    let app = FadeTopApp::<MockSampler>::new();

    let result = app.run(terminal);
    ratatui::restore();
    result
}
