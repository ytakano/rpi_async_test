#[allow(unused_imports)]
use diesel::prelude::*;

use crate::{perror, schema::*, EResult};
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
) -> EResult<()> {
    if let Err(e) = insert_into(data::table)
        .values((
            data::datetime.eq(dsl::now),
            data::temperature.eq(temperature),
            data::brightness.eq(brightness),
        ))
        .execute(conn)
    {
        perror!(e);
        return Err(e.into());
    }

    Ok(())
}

pub fn run(temp: Arc<AtomicU64>, bright: Arc<AtomicU64>) -> EResult<()> {
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
            let mut ts = [0.0; WINDOW_SIZE];
            let mut bs = [0.0; WINDOW_SIZE];
            let mut idx = 0;

            let f = move || {
                loop {
                    thread::sleep(wsec); // wsec秒待機

                    ts[idx] = f64::from_bits(temp.load(Ordering::Relaxed));
                    bs[idx] = f64::from_bits(bright.load(Ordering::Relaxed));
                    idx += 1;

                    if idx == WINDOW_SIZE {
                        // 平均値を挿入
                        let temp_ave = ts.iter().fold(0.0, |acc, n| acc + n) / WINDOW_SIZE as f64;
                        let bright_ave = bs.iter().fold(0.0, |acc, n| acc + n) / WINDOW_SIZE as f64;

                        if let Err(e) =
                            insert(&conn, Some(temp_ave as f32), Some(bright_ave as f32))
                        {
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
