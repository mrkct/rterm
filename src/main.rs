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

    #[clap(help="Data bits", default_value="8", value_parser=parse_data_bits)]
    data_bits: serialport::DataBits,

    #[clap(help="Parity", default_value="none", value_parser=parse_parity)]
    parity: serialport::Parity,

    #[clap(help="Stop bits", default_value="1", value_parser=parse_stop_bits)]
    stop_bits: serialport::StopBits,

    #[clap(help="Flow control", default_value="none", value_parser=parse_flow_control)]
    flow_control: serialport::FlowControl,
}

fn parse_data_bits(s: &str) -> Result<serialport::DataBits, String> {
    match s {
        "5" => Ok(serialport::DataBits::Five),
        "6" => Ok(serialport::DataBits::Six),
        "7" => Ok(serialport::DataBits::Seven),
        "8" => Ok(serialport::DataBits::Eight),
        _ => Err(format!("Invalid data bits: {}", s))
    }
}

fn parse_parity(s: &str) -> Result<serialport::Parity, String> {
    match s.to_lowercase().as_ref() {
        "none" => Ok(serialport::Parity::None),
        "even" => Ok(serialport::Parity::Even),
        "odd" => Ok(serialport::Parity::Odd),
        _ => Err(format!("Invalid parity: {}", s))
    }
}

fn parse_stop_bits(s: &str) -> Result<serialport::StopBits, String> {
    match s {
        "1" => Ok(serialport::StopBits::One),
        "2" => Ok(serialport::StopBits::Two),
        _ => Err(format!("Invalid stop bits: {}", s))
    }
}

fn parse_flow_control(s: &str) -> Result<serialport::FlowControl, String> {
    match s.to_lowercase().as_ref() {
        "none" => Ok(serialport::FlowControl::None),
        "hardware" => Ok(serialport::FlowControl::Hardware),
        "software" => Ok(serialport::FlowControl::Software),
        _ => Err(format!("Invalid flow control: {}", s))
    }
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
        .data_bits(args.data_bits)
        .parity(args.parity)
        .stop_bits(args.stop_bits)
        .flow_control(args.flow_control)
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
