#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!("bindings.rs");

#[allow(dead_code)]
mod dm2;

#[macro_use]
extern crate prometheus;

use futures::stream::StreamExt;
use getopts::Options;
use hyper;
use lazy_static::lazy_static;
use prometheus::{Counter, Encoder, GaugeVec, TextEncoder};
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::ffi::CStr;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::{fmt, str, time};
use tokio::sync::watch;
use warp::ws::{Message, WebSocket};
use warp::{Filter, Rejection, Reply};

#[derive(Debug)]
struct AppError {
    error: String,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl Error for AppError {
    fn description(&self) -> &str {
        &self.error
    }
}

#[derive(Debug)]
struct State {
    x: f32,

    // Latest gamepad state to reconcile to.
    gamepad_id: String,
    button_a: bool,
    axes_x: f32,
    axes_y: f32,
}

lazy_static! {
    static ref HTTP_COUNTER: Counter = register_counter!(opts!(
        "http_requests_total",
        "Total number of HTTP requests made.",
        labels! {"handler" => "all",}
    ))
    .unwrap();
    static ref ACCELERATION: GaugeVec =
        register_gauge_vec!("acceleration", "Acceleration in m/s^2", &["dimension"]).unwrap();

    // TODO: update servo position metrics.
    static ref SERVO: GaugeVec =
        register_gauge_vec!("servo", "Servo position", &["position"]).unwrap();

    // Shared state between C code and server
    static ref STATE: Mutex<State> = Mutex::new(State {
        x: 0.0,
        //gamepad: Gamepad {
        //    id: String::from(""),
        //    buttons: Vec::new(),
        //    axes: Vec::new(),
        //}
        gamepad_id: String::from("unknown"), // TODO: fix.
        button_a: false,
        axes_x: 0.0,
        axes_y: 0.0,
    });
}

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

static mut mpu_data: rc_mpu_data_t = rc_mpu_data_t {
    accel: [0.0; 3usize],
    gyro: [0.0; 3usize],
    mag: [0.0; 3usize],
    temp: 0.0,
    raw_gyro: [0; 3usize],
    raw_accel: [0; 3usize],
    __bindgen_padding_0: 0,
    accel_to_ms2: 0.0,
    gyro_to_degs: 0.0,
    dmp_quat: [0.0; 4usize],
    dmp_TaitBryan: [0.0; 3usize],
    tap_detected: 0,
    last_tap_direction: 0,
    last_tap_count: 0,
    __bindgen_padding_1: 0,
    fused_quat: [0.0; 4usize],
    fused_TaitBryan: [0.0; 3usize],
    compass_heading: 0.0,
    compass_heading_raw: 0.0,
};

// uart bus
//const bus: c_int = 0;
//const baudrate: c_int = 115200;

unsafe extern "C" fn mpu_callback() {
    ACCELERATION
        .with_label_values(&["x"])
        .set(mpu_data.accel[0]);
    ACCELERATION
        .with_label_values(&["y"])
        .set(mpu_data.accel[1]);
    ACCELERATION
        .with_label_values(&["z"])
        .set(mpu_data.accel[2]);
}

fn send_packet(bus: i32, data: &mut [u8]) -> Result<(), AppError> {
    let data_ptr = data.as_mut_ptr();
    unsafe {
        rc_uart_flush(bus);
        rc_uart_write(bus, data_ptr, data.len());
    }

    // Read what we wrote.
    let read_i = unsafe { rc_uart_read_bytes(bus, data_ptr, data.len()) };
    if read_i <= 0 {
        return Err(AppError {
            error: format!("packet read failed {}", read_i),
        });
    }

    if data[4] == dm2::ID_Broadcast {
        return Ok(());
    }

    // STATUS packet...
    let mut data: [u8; 32] = [0; 32];
    let data_ptr = data.as_mut_ptr();
    let read_i = unsafe { rc_uart_read_bytes(bus, data_ptr, data.len()) };
    if read_i <= 0 {
        return Err(AppError {
            error: format!("status packet read failed {}", read_i),
        });
    }
    if data[9] != 0 {
        return Err(AppError {
            error: format!("status packet error {:X?}", data[9]),
        });
    }
    println!("status {:X?}", data); // TODO: error check
    Ok(())
}

async fn run_events(signal: watch::Receiver<bool>, bus: i32, baud: i32) -> Result<(), AppError> {
    let c_str = unsafe {
        let s = rc_version_string();
        assert!(!s.is_null());

        CStr::from_ptr(s)
    };

    let r_str = c_str.to_str().unwrap();
    println!("version: {}", r_str);

    let adc_i = unsafe { rc_adc_init() };
    if adc_i != 0 {
        return Err(AppError {
            error: format!("ADC_init failed {}", adc_i),
        });
    };

    let batt_f = unsafe { rc_adc_batt() };
    if batt_f < 6.0 {
        return Err(AppError {
            error: format!("Low battery {}", batt_f),
        });
    };

    unsafe { rc_adc_cleanup() };

    println!("battery {}", batt_f);

    println!("initializing UART");
    let uart_i = unsafe { rc_uart_init(bus, baud, 0.5, 0, 1, 0) };
    if uart_i != 0 {
        return Err(AppError {
            error: format!("rc_uart_init failed {}", uart_i),
        });
    };

    println!("initializing DMP");
    let mut mpu_conf = unsafe { rc_mpu_default_config() };
    mpu_conf.dmp_sample_rate = 200; // Hertz
    mpu_conf.dmp_fetch_accel_gyro = 1;
    mpu_conf.orient = rc_mpu_orientation_t_ORIENTATION_Y_UP;

    //let gyro_callibrated = unsafe { rc_mpu_is_gyro_calibrated() };
    //if gyro_callibrated != 0 {
    //    return Err(AppError {
    //        error: format!("rc_mpu gyro not calibrated, run 'rc_calibrate_gyro' example program"),
    //    });
    //}

    let dmu_i = unsafe { rc_mpu_initialize_dmp(&mut mpu_data, mpu_conf) };
    if dmu_i != 0 {
        return Err(AppError {
            error: format!("dmp_init failed {}", dmu_i),
        });
    }

    // TODO: setup PID controllers
    let mut d1 = unsafe { rc_filter_empty() };
    let mut d2 = unsafe { rc_filter_empty() };
    let mut d3 = unsafe { rc_filter_empty() };

    // Safety: Call back into from C to rust...
    unsafe { rc_mpu_set_dmp_callback(Some(mpu_callback)) };

    // Setup xl320 motors
    let mut data = dm2::set_torque(dm2::ID_Broadcast, 0); // disable torque
    send_packet(bus, &mut data)?;
    //
    println!("CHECKING PING");
    for x in 1..3 {
        let mut data = dm2::ping(x as u8);
        send_packet(bus, &mut data)?;
    }

    let mut data = dm2::set_led(dm2::ID_Broadcast, 2); // green light
    send_packet(bus, &mut data)?;

    // Not sure what this does...
    let mut data = dm2::set_joint_mode(1, 1); // WHEEL Mode
    send_packet(bus, &mut data)?;

    // Disable angle limits for wheel mode?
    // https://github.com/ROBOTIS-GIT/DynamixelSDK/issues/129
    let mut data = dm2::set_angle_limit(1, 0, 0);
    send_packet(bus, &mut data)?;

    //let mut data = dm2::set_speed(dm2::ID_Broadcast, 0.0);
    //send_packet(bus, &mut data)?;

    println!("running...");
    let duration = time::Duration::from_micros(250000);
    while *signal.borrow() {
        tokio::time::delay_for(duration).await;

        let state = STATE.lock().unwrap();
        println!("STATE: {:?}", state);
        if !state.button_a {
            continue;
        }

        println!("SENDING: {}", state.axes_y);
        let mut data = dm2::set_pos(1, -state.axes_y);
        send_packet(bus, &mut data)?;

        let mut data = dm2::set_speed(2, state.axes_y);
        send_packet(bus, &mut data)?;

        /*println!("making packet");
        let mut data = dm2::set_led(dm2::ID_Broadcast, 2);
        //let mut data = dm2::test_packet();
        let data_ptr = data.as_mut_ptr();

        println!("writing packet");
        unsafe {
            rc_uart_flush(bus);
            rc_uart_write(bus, data_ptr, data.len());
        }
        println!("sent {:X?}", data);

        // Read
        println!("reading packet");
        let mut buf: [u8; 32] = [0; 32];
        let buf_ptr = buf.as_mut_ptr();
        let read_i = unsafe { rc_uart_read_bytes(bus, buf_ptr, 32) };
        if read_i <= 0 {
            return Err(AppError {
                error: format!("packet read failed {}", read_i),
            });
        }
        println!("packet {:X?}", buf);*/
    }
    let mut data = dm2::set_led(dm2::ID_Broadcast, 0);
    send_packet(bus, &mut data)?;

    let mut data = dm2::set_speed(dm2::ID_Broadcast, 0.0);
    send_packet(bus, &mut data)?;

    // Cleanup
    unsafe {
        rc_filter_free(&mut d1);
        rc_filter_free(&mut d2);
        rc_filter_free(&mut d3);
        rc_mpu_power_off();
        rc_uart_close(bus);
    };

    println!("shutdown loop");

    Ok(())
}

async fn metrics_handler() -> Result<impl Reply, Rejection> {
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    let encoder = TextEncoder::new();

    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        eprintln!("could not encode prometheus metrics: {}", e);
    };

    let val = hyper::header::HeaderValue::from_str(encoder.format_type()).unwrap();

    let mut res = hyper::Response::new(hyper::Body::from(buffer));
    res.headers_mut().insert(hyper::header::CONTENT_TYPE, val);
    Ok(res)
}

/// Our global unique user id counter.
static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Serialize, Deserialize, Debug)]
struct GamepadButton {
    value: f32,
    pressed: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct Gamepad {
    id: String,
    buttons: Vec<GamepadButton>,
    axes: Vec<f32>,
}

fn sock_message(_id: usize, msg: Message) {
    //let msg = msg.to_str().unwrap();
    let msg = if let Ok(s) = msg.to_str() {
        s
    } else {
        return;
    };

    //let out: Gamepad = serde_json::from_str(msg).unwrap();
    let pad: Gamepad = if let Ok(g) = serde_json::from_str(msg) {
        g
    } else {
        return;
    };
    //if pad.buttons[0].pressed {
    //    println!(
    //        "GAMEPAD: {} {} {} {}",
    //        pad.id, pad.buttons[0].pressed, pad.axes[0], pad.axes[1]
    //    );
    //}

    // TODO: update state...
    let mut state = STATE.lock().unwrap();
    if state.gamepad_id == pad.id {
        state.button_a = pad.buttons[0].pressed; // A -> true
        state.axes_x = pad.axes[0];
        state.axes_y = pad.axes[1];
    }
}

async fn sock_connected(ws: WebSocket) {
    // Assign a new id? Check we are id 1?
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    eprintln!("websocket connection: {}", id);

    // Split the socket into a sender and receive of messages.
    let (_, mut ws_rx) = ws.split();

    // Update the state with the new Gamepad.
    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(msg) => {
                sock_message(id, msg);
            }
            Err(e) => {
                eprintln!("websocket error(uid={}): {}", id, e);
                break;
            }
        };
    }
    eprintln!("websocket disconnect: {}", id); // disconnect
}

async fn ws_handler(ws: warp::ws::Ws) -> Result<impl Reply, Rejection> {
    Ok(ws.on_upgrade(move |socket| sock_connected(socket)))
}

async fn run_server(signal: impl Future<Output = ()> + Send + 'static) -> Result<(), AppError> {
    // GET /
    let index = warp::path::end().map(|| {
        let bytes = include_bytes!("index.html");
        let buf = hyper::body::Bytes::from_static(bytes);
        warp::reply::html(buf)
    });

    let script = warp::path("script.js").map(|| {
        let bytes = include_bytes!("script.js");
        let buf = hyper::body::Bytes::from_static(bytes);
        warp::reply::html(buf)
    });

    let metrics = warp::path!("metrics").and_then(metrics_handler);

    // POST /sock -> websocket upgrade
    let sock = warp::path("sock")
        // The `ws()` filter will prepare Websocket handshake...
        .and(warp::ws())
        .and_then(ws_handler);
    //.map(|ws: warp::ws::Ws| {
    // This will call our function if the handshake succeeds.
    //ws.on_upgrade(move |socket| sock_connected(socket))
    //ws.on_upgrade(move |socket| async move {
    //    tokio::spawn(sock_connected(socket));
    //})
    //});

    let (_, server) = warp::serve(index.or(script).or(metrics).or(sock))
        .bind_with_graceful_shutdown(([0, 0, 0, 0], 8080), signal);

    server.await;
    println!("shutdown server");
    Ok(())

    //   if let Err(e) = server.await {
    //       Err(AppError {
    //           error: format!("server: {}", e),
    //       })
    //   } else {
    //   }
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {}", program);
    print!("{}", opts.usage(&brief));
}

#[tokio::main]
#[cfg(all(target_os = "linux"))]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("z", "baud", "set baud rate", "115200");
    opts.optopt("b", "bus", "set uart bus", "1");
    opts.optflag("h", "help", "print this help menu");
    opts.optopt(
        "i",
        "id",
        "set gamepad id",
        "Pro Controller (STANDARD GAMEPAD Vendor: 057e Product: 2009)",
    );
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let baud: i32 = match matches.opt_str("z") {
        Some(m) => m.parse().unwrap(),
        _ => 115200,
    };
    let bus: i32 = match matches.opt_str("b") {
        Some(m) => m.parse().unwrap(),
        _ => 1,
    };
    match matches.opt_str("i") {
        Some(id) => {
            let mut state = STATE.lock().unwrap();
            state.gamepad_id = id
        }
        _ => (),
    };

    let (tx, rx) = watch::channel(true);

    let cancel = async move {
        shutdown_signal().await;
        tx.broadcast(false).expect("failed to broadcast CTRL+C");
    };

    let server = run_server(cancel);

    let events = run_events(rx, bus, baud);

    let res = tokio::try_join!(server, events);

    match res {
        Ok(_) => {
            println!("end");
        } // Do nothing
        Err(err) => {
            eprintln!("error: {}", err);
            std::process::exit(1);
        }
    }
}

#[cfg(any(not(target_os = "linux")))]
fn main() {
    println!(r#"Invalid compile target"#);
}
