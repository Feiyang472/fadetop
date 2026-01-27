pub use remoteprocess::Pid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalVariable {
    pub name: String,
    pub repr: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub name: String,
    pub filename: String,
    pub locals: Option<Vec<LocalVariable>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackTrace {
    pub thread_id: u64,
    pub pid: Pid,
    pub frames: Vec<Frame>,
    pub thread_name: Option<String>,
}

impl From<&py_spy::stack_trace::LocalVariable> for LocalVariable {
    fn from(local: &py_spy::stack_trace::LocalVariable) -> Self {
        LocalVariable {
            name: local.name.clone(),
            repr: local.repr.clone(),
        }
    }
}

impl From<&py_spy::stack_trace::Frame> for Frame {
    fn from(frame: &py_spy::stack_trace::Frame) -> Self {
        Frame {
            name: frame.name.clone(),
            filename: frame.filename.clone(),
            locals: frame
                .locals
                .as_ref()
                .map(|locals| locals.iter().map(LocalVariable::from).collect()),
        }
    }
}

impl From<&py_spy::stack_trace::StackTrace> for StackTrace {
    fn from(trace: &py_spy::stack_trace::StackTrace) -> Self {
        StackTrace {
            thread_id: trace.thread_id,
            pid: trace.pid,
            frames: trace.frames.iter().map(Frame::from).collect(),
            thread_name: trace.thread_name.clone(),
        }
    }
}
