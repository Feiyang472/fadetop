use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;

use anyhow::Error;

use crate::app::SamplerOps;
use crate::config::AppConfig;
use crate::priority::SpiedRecordQueueMap;
use crate::processes::{ProcessDiscovery, PythonProcess};
use crate::sample::{Frame, LocalVariable, Pid, StackTrace};
use crate::tabs::process_selection::ProcessSelectionState;

/// Mock process discovery that returns dummy processes for the demo
pub struct MockProcessDiscovery;

impl ProcessDiscovery for MockProcessDiscovery {
    fn discover() -> Vec<PythonProcess> {
        vec![
            PythonProcess {
                pid: 1001,
                cmdline: "python3 /home/user/scripts/web_server.py --port 8080".to_string(),
            },
            PythonProcess {
                pid: 1002,
                cmdline: "python3 -m pytest tests/".to_string(),
            },
            PythonProcess {
                pid: 1003,
                cmdline: "python /usr/bin/jupyter notebook".to_string(),
            },
            PythonProcess {
                pid: 2001,
                cmdline: "python3 data_pipeline.py --input data.csv".to_string(),
            },
            PythonProcess {
                pid: 3456,
                cmdline: "python3 -c 'import time; time.sleep(3600)'".to_string(),
            },
        ]
    }
}

#[derive(Clone, Debug)]
pub struct MockSampler {
    stop_flag: Arc<AtomicBool>,
}

impl Default for MockSampler {
    fn default() -> Self {
        Self {
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Drop for MockSampler {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

impl SamplerOps for MockSampler {
    fn from_config_and_id(_config: &AppConfig, _pid: Pid) -> Result<Self, Error> {
        Ok(MockSampler::default())
    }

    async fn push_to_queue(self, queue: Arc<RwLock<SpiedRecordQueueMap>>) -> Result<(), Error> {
        let stop_flag = Arc::clone(&self.stop_flag);
        tokio::task::spawn_blocking(move || Self::push_to_queue_sync(queue, stop_flag)).await?
    }
}

impl MockSampler {
    fn push_to_queue_sync(
        queue: Arc<RwLock<SpiedRecordQueueMap>>,
        stop_flag: Arc<AtomicBool>,
    ) -> Result<(), Error> {
        while !stop_flag.load(Ordering::Relaxed) {
            for pid in 0..10 {
                let frame_template = Frame {
                    name: "level0".to_string(),
                    filename: "lorem/ipsum/dolor/sit/amet/consectetur/adipiscing/elit/test.py"
                        .to_string(),
                    locals: Some(vec![
                        LocalVariable {
                            name: "x".to_string(),
                            repr: Some("data, verryyyyyy looonnnnnnng data".to_string()),
                        },
                        LocalVariable {
                            name: "είναι απλά ένα κείμενο".to_string(),
                            repr: Some(
                                "χωρίς νόημα για τους επαγγελματίες της τυπογραφίας ".to_string(),
                            ),
                        },
                    ]),
                };

                let trace = StackTrace {
                    thread_id: pid * 10 + 1,
                    pid: pid as Pid,
                    frames: vec![
                        Frame {
                            name: "level1".to_string(),
                            ..frame_template.clone()
                        },
                        frame_template.clone(),
                    ],
                    thread_name: Some("Main Thread".into()),
                };

                for _ in 0..10 {
                    if stop_flag.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                    thread::sleep(std::time::Duration::from_millis(10));
                    queue.write().unwrap().increment(&trace);
                }

                for _ in 0..10 {
                    if stop_flag.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                    thread::sleep(std::time::Duration::from_millis(10));
                    queue.write().unwrap().increment(&trace);
                }
                for _ in 0..10 {
                    if stop_flag.load(Ordering::Relaxed) {
                        return Ok(());
                    }
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

                if stop_flag.load(Ordering::Relaxed) {
                    return Ok(());
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
                    if stop_flag.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                    thread::sleep(std::time::Duration::from_millis(10));
                    queue.write().unwrap().increment(&StackTrace {
                        frames: vec![Frame {
                            name: "level2_different".to_string(),
                            ..frame_template.clone()
                        }],
                        thread_id: pid * 10 + 2,
                        thread_name: Some("Worker Thread".into()),
                        ..trace.clone()
                    });
                }
            }
        }
        Ok(())
    }
}

pub async fn run() -> Result<(), Error> {
    let configs = AppConfig::from_configs()?;
    let terminal = ratatui::init();

    let result = ProcessSelectionState::new()
        .run_with_selection::<MockSampler, MockProcessDiscovery>(&configs, terminal)
        .await;
    ratatui::restore();
    result
}
