pub(super) struct ThreadPool {
    sender: crossbeam_channel::Sender<Box<dyn Fn() + Send + 'static>>,
    // the handles are joined on drop; this *must* be after `sender`
    _handles: Box<[Handle]>,
}

impl ThreadPool {
    pub(super) fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let num_threads = num_cpus::get();
        let mut handles = Vec::new();
        for _ in 0..num_threads {
            let join_handle = std::thread::spawn({
                let receiver: crossbeam_channel::Receiver<Box<dyn Fn() + Send + 'static>> =
                    receiver.clone();
                move || {
                    for job in receiver {
                        job();
                    }
                }
            });
            handles.push(Handle { inner: Some(join_handle) });
        }
        Self { sender, _handles: handles.into_boxed_slice() }
    }

    pub(super) fn spawn(&self, job: impl Fn() + Send + 'static) {
        self.sender.send(Box::new(job)).expect("failed to send job to thread pool");
    }
}

struct Handle {
    inner: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Handle {
    fn drop(&mut self) {
        if let Some(handle) = self.inner.take()
            && let Err(e) = handle.join()
            && !std::thread::panicking()
        {
            panic!("failed to join thread handle: {e:?}");
        }
    }
}

#[test]
fn smoke() {
    let values = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    {
        let pool = ThreadPool::new();
        pool.spawn({
            let values = std::sync::Arc::clone(&values);
            move || {
                std::thread::sleep(std::time::Duration::from_millis(20));
                values.lock().unwrap().push("def");
            }
        });
        pool.spawn({
            let values = std::sync::Arc::clone(&values);
            move || {
                std::thread::sleep(std::time::Duration::from_millis(30));
                values.lock().unwrap().push("abc");
            }
        });
        pool.spawn({
            let values = std::sync::Arc::clone(&values);
            move || {
                std::thread::sleep(std::time::Duration::from_millis(10));
                values.lock().unwrap().push("ghi");
            }
        });
    }
    let mut values = std::sync::Arc::try_unwrap(values).unwrap().into_inner().unwrap();
    values.sort_unstable();
    assert_eq!(values, ["abc", "def", "ghi"]);
}
