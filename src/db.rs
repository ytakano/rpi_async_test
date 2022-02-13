#[allow(unused_imports)]
use diesel::prelude::*;

use crate::{perror, schema::*, Air, EResult};
use diesel::{dsl, insert_into, PgConnection};
use std::{
    env,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

const ENV_STR: &str = "DATABASE_URL";
const WINDOW_SIZE: usize = 5;

pub fn insert(
    conn: &PgConnection,
    temperature: Option<f32>,
    brightness: Option<f32>,
    co2: Option<i32>,
    tvoc: Option<i32>,
) -> EResult<()> {
    if let Err(e) = insert_into(data::table)
        .values((
            data::datetime.eq(dsl::now),
            data::temperature.eq(temperature),
            data::brightness.eq(brightness),
            data::co2.eq(co2),
            data::tvoc.eq(tvoc),
        ))
        .execute(conn)
    {
        perror!(e);
        return Err(e.into());
    }

    Ok(())
}

pub fn run(air: Air, bright: Arc<AtomicU64>) -> EResult<()> {
    let url = match env::var(ENV_STR) {
        Ok(s) => s,
        Err(e) => {
            perror!(e);
            return Err(e.into());
        }
    };

    match PgConnection::establish(&url) {
        Ok(conn) => {
            let wsec = Duration::from_secs(1);
            let mut temp_v = [0.0; WINDOW_SIZE];
            let mut bright_v = [0.0; WINDOW_SIZE];
            let mut co2_v = [0; WINDOW_SIZE];
            let mut tvoc_v = [0; WINDOW_SIZE];
            let mut idx = 0;

            let f = move || {
                loop {
                    thread::sleep(wsec); // wsec秒待機

                    temp_v[idx] = f64::from_bits(air.temp.load(Ordering::Relaxed));
                    bright_v[idx] = f64::from_bits(bright.load(Ordering::Relaxed));
                    co2_v[idx] = air.co2.load(Ordering::Relaxed);
                    tvoc_v[idx] = air.tvoc.load(Ordering::Relaxed);
                    idx += 1;

                    if idx == WINDOW_SIZE {
                        // 平均値
                        let temp_ave =
                            temp_v.iter().fold(0.0, |acc, n| acc + n) / WINDOW_SIZE as f64;
                        let bright_ave =
                            bright_v.iter().fold(0.0, |acc, n| acc + n) / WINDOW_SIZE as f64;

                        // 中央値
                        co2_v.sort();
                        tvoc_v.sort();
                        let co2 = co2_v[WINDOW_SIZE >> 1];
                        let tvoc = tvoc_v[WINDOW_SIZE >> 1];

                        // 挿入
                        if let Err(e) = insert(
                            &conn,
                            Some(temp_ave as f32),
                            Some(bright_ave as f32),
                            Some(co2 as i32),
                            Some(tvoc as i32),
                        ) {
                            perror!(e);
                            break;
                        }

                        println!("inserted to DB");

                        idx = 0;
                    }
                }
            };

            thread::spawn(f);

            Ok(())
        }
        Err(e) => {
            perror!(e);
            Err(e.into())
        }
    }
}
