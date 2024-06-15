use clap::Parser;

use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use device_query::{DeviceEvents, DeviceState};


#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[clap(help="Serial port to use")]
    path: String,

    #[clap(help="Baudrate to use", default_value="115200")]
    baudrate: u32,
}

#[cfg(target_family = "unix")]
fn disable_terminal_echo()
{
    let stdout = io::stdout().as_raw_fd();

    let original = termios::Termios::from_fd(stdout).unwrap();
    let _ = ctrlc::set_handler(move || {
        restore_terminal_echo(original);
        std::process::exit(0);
    });
    
    let mut termios = original.clone();
    termios::tcgetattr(stdout, &mut termios).unwrap();
    termios.c_lflag &=!(termios::ICANON | termios::ECHO);
    termios::tcsetattr(stdout, termios::TCSANOW, &termios).unwrap();
}

#[cfg(target_family = "unix")]
fn restore_terminal_echo(original: termios::Termios)
{
    let stdout = io::stdout().as_raw_fd();
    termios::tcsetattr(stdout, termios::TCSANOW, &original).unwrap();
}

fn main() {
    let args = Args::parse();
    let mut port = serialport::new(args.path, args.baudrate)
        .timeout(Duration::from_millis(10))
        .open()
        .expect("Failed to open port");

    

    let device_state = DeviceState::new();
    let port_writer = Arc::new(Mutex::new(port.try_clone().unwrap()));
    disable_terminal_echo();
    
    let writer1 = port_writer.clone();
    let _guard = device_state.on_key_down(move |key| {
        if let Ok(mut port) = writer1.lock() {
            let _ = port.write(&[0x55, 1, *key as u8]);
        }
    });

    let writer2 = port_writer.clone();
    let _guard = device_state.on_key_up(move |key| {
        if let Ok(mut port) = writer2.lock() {
            let _ = port.write(&[0x55, 0, *key as u8]);
        }
    });

    let mut serial_buf: Vec<u8> = vec![0; 1000];
    loop {
        match port.read(serial_buf.as_mut_slice()) {
            Ok(t) => {
                io::stdout().write_all(&serial_buf[..t]).unwrap();
                io::stdout().flush().unwrap();
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            Err(e) => {
                eprintln!("{:?}", e);
                break;
            }
        }
    }
}
