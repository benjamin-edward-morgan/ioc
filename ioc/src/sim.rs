use crate::InputSource;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::task::JoinHandle;
use tracing::warn;

pub mod second_order_ode;

#[derive(Debug)]
struct ValueAverageData {
    last_value: f64,
    last_instant: Instant,
    sum: f64,
    start_instant: Instant,
}

impl ValueAverageData {
    pub fn new(start: f64) -> Self {
        let now = Instant::now();
        ValueAverageData {
            last_value: start,
            last_instant: now,
            sum: 0.0,
            start_instant: now,
        }
    }

    pub fn append(&mut self, new_value: f64) {
        let now = Instant::now();

        //append a reimann sum to sum, set last_value and last_instant
        let since_last_sec = now.duration_since(self.last_instant).as_secs_f64();
        self.sum += since_last_sec * self.last_value;
        self.last_value = new_value;
        self.last_instant = now;
    }

    pub fn read(&mut self) -> f64 {
        //calculate the reimann sum of the value since the last reading, divide by total time to get average
        let now = Instant::now();
        let total_time_sec = now.duration_since(self.start_instant).as_secs_f64();
        let average: f64 = if total_time_sec > 0.0 {
            let last_time_sec = now.duration_since(self.last_instant).as_secs_f64();
            let integral = self.sum + (last_time_sec * self.last_value);
            integral / total_time_sec
        } else {
            //if no time has elapsed, just use the last_value
            self.last_value
        };

        //reset
        self.last_instant = now;
        self.sum = 0.0;
        self.start_instant = now;

        //return average since last reading
        average
    }
}

pub struct ValueAverage {
    data: Arc<Mutex<ValueAverageData>>,
    pub handle: JoinHandle<()>,
}

impl ValueAverage {
    pub fn new(source: InputSource<f64>) -> Self {
        let data = Arc::new(Mutex::new(ValueAverageData::new(source.start)));

        let mut rx = source.rx;
        let handle_data = Arc::clone(&data);
        let handle = tokio::spawn(async move {
            loop {
                match (rx.recv().await, handle_data.lock()) {
                    (Ok(new_value), Ok(mut data)) => {
                        data.append(new_value);
                    }
                    (val_res, mtx_res) => {
                        warn!(
                            "value avg error!\nreceived:{:?}\ndata_locl:{:?}",
                            val_res, mtx_res
                        );
                    }
                }
            }
        });

        ValueAverage { data, handle }
    }

    pub fn read(&mut self) -> f64 {
        match self.data.lock() {
            Ok(mut data) => data.read(),
            mtx_res => {
                warn!("value avg read error! {:?}", mtx_res);
                0.0
            }
        }
    }
}
