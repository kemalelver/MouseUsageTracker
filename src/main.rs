use chrono::{DateTime, Utc};
use rdev::{listen, Event, EventType};
use rusqlite::{Connection, Result as SqlResult};
use tokio::sync::Mutex;
use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, thread};
use std::sync::mpsc;
use tray_item::{TrayItem, IconSource};

enum Message {
    Quit,
    Update  
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = Arc::new(Mutex::new(Connection::open("mouse_usage.db")?));

    initialize_database(&conn).await?;

    let total_usage = Arc::new(Mutex::new(load_last_usage_duration(&conn).await?));
    let last_activity = Arc::new(Mutex::new(Utc::now()));
    let last_logging_time = Arc::new(Mutex::new(Utc::now()));
    let running = Arc::new(AtomicBool::new(true));

    let listener_thread = tokio::spawn({
        let activity_last = Arc::clone(&last_activity);
        let mut lastspawn = Utc::now();
        async move {
            listen(move |event: Event| {
                match event.event_type {
                    EventType::MouseMove { .. } => {
                        if (Utc::now() - lastspawn).num_milliseconds()  > 850 {
                        tokio::spawn(update_last_activity(Arc::clone(&activity_last)));
                        lastspawn = Utc::now();
                        }
                    }
                    EventType::ButtonPress(_) | EventType::ButtonRelease(_) => {
                        tokio::spawn(update_last_activity(Arc::clone(&activity_last)));
                    }
                    EventType::Wheel { .. } => {
                        tokio::spawn(update_last_activity(Arc::clone(&activity_last)));
                    }
                    _ => {}
                }
            })
            .unwrap();
        }
    });

    let running_logger = Arc::clone(&running);
    let logging_thread = tokio::spawn({
        let usage_total = Arc::clone(&total_usage);
        let last_activity_clone = Arc::clone(&last_activity);
        let last_logging_time_clone = Arc::clone(&last_logging_time);
        async move {
            while running_logger.load(Ordering::SeqCst) {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                let duration = {
                    let activity_last_value = last_activity_clone.lock().await;
                    let last_logging_time = last_logging_time.lock().await;
                    *activity_last_value - *last_logging_time 
                };

                if duration.num_seconds() < 10 && duration.num_seconds() > 0 {
                    let mut total_usage = usage_total.lock().await;                    
                    *total_usage += duration.num_nanoseconds().unwrap() as f64 * 1e-9;
                    let total_usage_seconds = *total_usage; // MutexGuard<'_, f64> türünü f64'e dönüştürme
                    
                    if let Err(e) = log_usage(&conn, total_usage_seconds).await {
                        eprintln!("Error logging usage: {}", e);
                    }
                }

                let mut last_logging_time_clone = last_logging_time_clone.lock().await;
                *last_logging_time_clone = Utc::now();
            }
        }
    });

    let tray = Arc::new(Mutex::new(
        TrayItem::new("Mouse Usage Tracker", IconSource::Resource("tray-default")).unwrap(),
    ));

    // Locking the tray to access or modify it (await the lock)
    tray.lock().await
        .add_label("Mouse Usage Tracker is running")
        .unwrap();

    let label_id = tray.lock().await
        .inner_mut()
        .add_label_with_id("Tray Label")
        .unwrap();

    let (tx, rx) = mpsc::sync_channel(1);

    tray.lock().await.inner_mut().add_separator().unwrap();

    let quit_tx = tx.clone();
    tray.lock().await.add_menu_item("Quit", move || {
        quit_tx.send(Message::Quit).unwrap();
    }).unwrap();

    let usage_total_2 = Arc::clone(&total_usage);
    let tray_clone = Arc::clone(&tray);

    let label_update_thread = tokio::spawn(async move {
        loop {
            let total_usage_seconds = {
                let usage_total_2 = usage_total_2.lock().await;
                *usage_total_2
            };

            let usage_duration_hours = total_usage_seconds / 3600.0;

            // Locking the tray before accessing its inner parts (await the lock)
            let mut tray = tray_clone.lock().await;
            tray.inner_mut()
                .set_label(&usage_duration_hours.to_string(), label_id)
                .unwrap();

                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        }
    });

    tokio::spawn(async move {
        loop {
            match rx.recv() {
                Ok(Message::Quit) => {
                    println!("Quit");
                    std::process::abort();
                }
                _ => {}
            }
        }
    });


    label_update_thread.await.unwrap();

    listener_thread.await?;
    logging_thread.await?;

    Ok(())
}

async fn update_last_activity(activity_last: Arc<Mutex<DateTime<Utc>>>) {
    let mut last_activity = activity_last.lock().await;
    *last_activity = Utc::now();
}

async fn initialize_database(conn: &Arc<Mutex<Connection>>) -> SqlResult<()> {
    let conn = conn.lock().await;
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS usage_log_rust (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT,
            usage_duration_seconds REAL,
            usage_duration_hours REAL
        )
        "#,
        [],
    )?;
    Ok(())
}

async fn log_usage(conn: &Arc<Mutex<Connection>>, usage_duration_seconds: f64) -> SqlResult<()> {
    let usage_duration_hours = usage_duration_seconds / 3600.0;
    let current_time = Utc::now().naive_utc().to_string();

    let conn = conn.lock().await;
    conn.execute(
        r#"
        INSERT INTO usage_log_rust (timestamp, usage_duration_seconds, usage_duration_hours)
        VALUES (?1, ?2, ?3)
        "#,
        rusqlite::params![current_time, usage_duration_seconds, usage_duration_hours],
    )?;
    Ok(())
}

async fn load_last_usage_duration(conn: &Arc<Mutex<Connection>>) -> SqlResult<f64> {
    let conn = conn.lock().await;
    let mut stmt = conn.prepare(
        r#"
        SELECT usage_duration_seconds
        FROM usage_log_rust
        ORDER BY id DESC
        LIMIT 1
        "#,
    )?;

    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        Ok(row.get(0)?)
    } else {
        Ok(0.0)
    }
}
