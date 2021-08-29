use std::thread;
use std::sync::{mpsc, Arc, Mutex};

pub struct Pool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    Work(Job),
    Stop,
}

impl Pool {
    pub fn new(size: usize) -> Self {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);
        for _ in 0..size {
            workers.push(Worker::new(Arc::clone(&receiver)));
        }

        Pool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
        where F: FnOnce() + Send + 'static
    {
        self.sender.send(Message::Work(Box::new(f))).unwrap();
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        for _ in &self.workers {
            self.sender.send(Message::Stop).unwrap();
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Self {
        let thread = thread::spawn(move ||
            loop {
                let message = receiver.lock().unwrap().recv().unwrap();

                match message {
                    Message::Work(job) => {
                        job();
                    }
                    Message::Stop => {
                        break;
                    }
                }
            }
        );

        Worker { thread: Some(thread) }
    }
}
