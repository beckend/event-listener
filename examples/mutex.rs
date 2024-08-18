//! A simple mutex implementation.
//!
//! This mutex exposes both blocking and async methods for acquiring a lock.

#[cfg(not(target_family = "wasm"))]
mod example {
    #![allow(dead_code)]

    use std::collections::VecDeque;
    use std::ops::{Deref, DerefMut};
    use std::sync::Arc;
    use std::thread::{available_parallelism, scope};
    use std::time::{Duration, Instant};

    use event_listener::{listener, Event, Listener};
    use try_lock::{Locked, TryLock};

    /// A simple mutex.
    struct Mutex<T> {
        /// Blocked lock operations.
        lock_ops: Event,

        /// The inner non-blocking mutex.
        data: TryLock<T>,
    }

    unsafe impl<T: Send> Send for Mutex<T> {}
    unsafe impl<T: Send> Sync for Mutex<T> {}

    impl<T> Mutex<T> {
        /// Creates a mutex.
        fn new(t: T) -> Mutex<T> {
            Mutex {
                lock_ops: Event::new(),
                data: TryLock::new(t),
            }
        }

        /// Attempts to acquire a lock.
        fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
            self.data.try_lock().map(MutexGuard)
        }

        /// Blocks until a lock is acquired.
        fn lock(&self) -> MutexGuard<'_, T> {
            loop {
                // Attempt grabbing a lock.
                if let Some(guard) = self.try_lock() {
                    return guard;
                }

                // Set up an event listener.
                listener!(self.lock_ops => listener);

                // Try again.
                if let Some(guard) = self.try_lock() {
                    return guard;
                }

                // Wait for a notification.
                listener.wait();
            }
        }

        /// Blocks until a lock is acquired or the timeout is reached.
        fn lock_timeout(&self, timeout: Duration) -> Option<MutexGuard<'_, T>> {
            let deadline = Instant::now() + timeout;

            loop {
                // Attempt grabbing a lock.
                if let Some(guard) = self.try_lock() {
                    return Some(guard);
                }

                // Set up an event listener.
                listener!(self.lock_ops => listener);

                // Try again.
                if let Some(guard) = self.try_lock() {
                    return Some(guard);
                }

                // Wait until a notification is received.
                listener.wait_deadline(deadline)?;
            }
        }

        /// Acquires a lock asynchronously.
        async fn lock_async(&self) -> MutexGuard<'_, T> {
            loop {
                // Attempt grabbing a lock.
                if let Some(guard) = self.try_lock() {
                    return guard;
                }

                // Set up an event listener.
                listener!(self.lock_ops => listener);

                // Try again.
                if let Some(guard) = self.try_lock() {
                    return guard;
                }

                // Wait until a notification is received.
                listener.await;
            }
        }
    }

    /// A guard holding a lock.
    struct MutexGuard<'a, T>(Locked<'a, T>);

    impl<T> Deref for MutexGuard<'_, T> {
        type Target = T;

        fn deref(&self) -> &T {
            &self.0
        }
    }

    impl<T> DerefMut for MutexGuard<'_, T> {
        fn deref_mut(&mut self) -> &mut T {
            &mut self.0
        }
    }

    pub(super) fn entry() {
        let count_max = 10000_usize;
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let thread_count = available_parallelism().unwrap().get() * 4;
        let thread_loop = count_max / thread_count;
        let mut count_actual = 0_usize;

        scope(|s| {
            for _ in 0..thread_count {
                let queue = queue.clone();
                count_actual += thread_loop;

                s.spawn(move || {
                    for i in 0..thread_loop {
                        queue.lock().push_back(i);
                    }
                });
            }
        });

        assert_eq!(queue.lock().len(), count_actual);

        println!("Done!");
    }
}

#[cfg(target_family = "wasm")]
mod example {
    pub(super) fn entry() {
        println!("This example is not supported on wasm yet.");
    }
}

fn main() {
    example::entry();
}
