use alloy_json_rpc::{RpcParam, RpcReturn};
use alloy_transport::Transport;
use core::panic;
use futures::{stream, Stream};
use ic_cdk_timers::{set_timer_interval, TimerId};
use serde::Serialize;
use serde_json::value::RawValue;
use std::{
    borrow::Cow,
    marker::PhantomData,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use crate::WeakClient;

/// ...
#[derive(Debug)]
pub struct IcpPollerBuilder<Conn, Params, Resp> {
    client: WeakClient<Conn>,
    _pd: PhantomData<fn() -> Resp>,
    method: Cow<'static, str>,
    params: Params,
    poll_interval: Duration,
    limit: usize,
    timer_id: Option<TimerId>,
}

impl<Conn, Params, Resp> IcpPollerBuilder<Conn, Params, Resp>
where
    Conn: Transport + Clone + 'static,
    Params: RpcParam + 'static,
    Resp: RpcReturn + Clone + 'static,
{
    /// Create a new poller task.
    pub fn new(
        client: WeakClient<Conn>,
        method: impl Into<Cow<'static, str>>,
        params: Params,
    ) -> Self {
        let poll_interval =
            client.upgrade().map_or_else(|| Duration::from_secs(7), |c| c.poll_interval());
        Self {
            client,
            method: method.into(),
            params,
            timer_id: None,
            _pd: PhantomData,
            poll_interval,
            limit: usize::MAX,
        }
    }

    /// Returns the limit on the number of successful polls.
    pub const fn limit(&self) -> usize {
        self.limit
    }

    /// Sets a limit on the number of successful polls.
    pub fn set_limit(&mut self, limit: Option<usize>) {
        self.limit = limit.unwrap_or(usize::MAX);
    }

    /// Sets a limit on the number of successful polls.
    pub fn with_limit(mut self, limit: Option<usize>) -> Self {
        self.set_limit(limit);
        self
    }

    /// Returns the duration between polls.
    pub const fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    /// Sets the duration between polls.
    pub fn set_poll_interval(&mut self, poll_interval: Duration) {
        self.poll_interval = poll_interval;
    }

    /// Sets the duration between polls.
    pub fn with_poll_interval(mut self, poll_interval: Duration) -> Self {
        self.set_poll_interval(poll_interval);
        self
    }

    /// ...
    pub fn start<F>(mut self, response_handler: F) -> Result<TimerId, String>
    where
        F: FnMut(Resp) + Send + Sync + 'static,
    {
        let client = match WeakClient::upgrade(&self.client) {
            Some(c) => c,
            None => return Err("Client has been dropped.".into()),
        };

        let timer_id = Arc::new(Mutex::new(None));

        let poll = {
            let timer_id = Arc::clone(&timer_id);
            let response_handler = Arc::new(Mutex::new(response_handler));
            let poll_count = Arc::new(AtomicUsize::new(0));

            move || {
                ic_cdk::spawn({
                    let response_handler = Arc::clone(&response_handler);
                    let poll_count = Arc::clone(&poll_count);
                    let timer_id = Arc::clone(&timer_id);
                    let mut params = ParamsOnce::Typed(self.params.clone());
                    let client = Arc::clone(&client);
                    let method = self.method.clone();

                    async move {
                        let params = match params.get() {
                            Ok(p) => p,
                            Err(e) => {
                                ic_cdk::println!("Failed to get params: {:?}", e);
                                return;
                            }
                        };

                        ic_cdk::println!("RPC request");
                        let result = client.request(method, params).await;

                        match result {
                            Ok(response) => {
                                let count = poll_count.fetch_add(1, Ordering::SeqCst) + 1;

                                match response_handler.lock() {
                                    Ok(mut handler) => handler(response),
                                    Err(e) => ic_cdk::println!(
                                        "Failed to acquire lock on response handler: {:?}",
                                        e
                                    ),
                                }

                                if count >= self.limit {
                                    if let Some(timer_id) = *timer_id.lock().unwrap() {
                                        ic_cdk_timers::clear_timer(timer_id);
                                    }
                                }
                            }
                            Err(e) => ic_cdk::println!("Request failed: {:?}", e),
                        }
                    }
                });
            }
        };

        // Initial poll
        poll();

        // Subsequent polls
        let id = set_timer_interval(self.poll_interval, poll);
        let mut timer_id_lock = timer_id.lock().unwrap();
        *timer_id_lock = Some(id);
        self.timer_id = Some(id);

        Ok(id)
    }

    /// ...
    pub fn stop(&mut self) {
        if let Some(timer_id) = self.timer_id.take() {
            ic_cdk_timers::clear_timer(timer_id);
        }
    }

    /// ...
    #[allow(unreachable_code)]
    pub fn into_stream(self) -> impl Stream<Item = Resp> + Unpin {
        panic!("Streams cannot be used ICP canisters.");
        stream::empty()
    }
}

// Serializes the parameters only once.
enum ParamsOnce<P> {
    Typed(P),
    Serialized(Box<RawValue>),
}

impl<P: Serialize> ParamsOnce<P> {
    #[inline]
    fn get(&mut self) -> serde_json::Result<&RawValue> {
        match self {
            Self::Typed(_) => self.init(),
            Self::Serialized(p) => Ok(p),
        }
    }

    #[cold]
    fn init(&mut self) -> serde_json::Result<&RawValue> {
        let Self::Typed(p) = self else { unreachable!() };
        let v = serde_json::value::to_raw_value(p)?;
        *self = Self::Serialized(v);
        let Self::Serialized(v) = self else { unreachable!() };
        Ok(v)
    }
}
