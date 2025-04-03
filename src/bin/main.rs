use std::{
    env, fs,
    io::Write,
    net::{TcpListener, TcpStream},
    process::Command,
    thread,
    time::Duration,
};

use rust_threadpool::ThreadPool;

// cargo run 500 1000 1033

fn main() {
    let pool = ThreadPool::new(4);
    let port = read_port();
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
    println!("Server is ready at 127.0.0.1:{port}");
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        pool.execute(move || {
            write_to_stream(stream);
        });
    }
}

enum ArgumentsOrder {
    // ExecutableName = 0,
    RateChunk = 1,
    RateTime = 2,
    Port = 3,
    Filename = 4,
    Repeat = 5,
    MaxBytes = 6,
}

struct CommandLineArguments {
    content: String,
    rate_chunk: usize,
    rate_time: u64,
    repeat: bool,
    max_bytes: Option<usize>,
}

fn create_file() -> String {
    let rust_cwd = env::current_dir().unwrap();
    let destination = rust_cwd.to_str().unwrap().to_owned() + "/target/raw.data";
    if fs::exists(destination.as_str()).unwrap() == false {
        env::set_current_dir("../../Zig/zig_003_random_float_raw_data/")
            .expect("Change cwd failed");
        let zig_cwd = env::current_dir().unwrap();
        let mut zig = Command::new("zig");
        zig.args(vec!["build", "run"])
            .status()
            .expect("Error executing zig");
        fs::copy(
            zig_cwd.to_str().unwrap().to_owned() + "/zig-out/bin/raw.data",
            destination.as_str(),
        )
        .expect("Copy Error");
        env::set_current_dir(rust_cwd).unwrap();
    }
    return destination;
}

fn validate_args() -> Vec<String> {
    let args: Vec<String> = env::args().collect();
    const MIN_ARG_COUNT: usize = 3;
    const MAX_ARG_COUNT: usize = 6;
    if args.len() < MIN_ARG_COUNT + 1 || args.len() > MAX_ARG_COUNT + 1 {
        panic!(
            "Should have at least {MIN_ARG_COUNT} arguments and most {MAX_ARG_COUNT} arguments:\n
            rate_chunk [bytes] rate_time [ms] port [0-65535]\n
            filename [path (Default: /target/raw.data)]\n
            repeat [true or false (Default: false)]\n
            max_bytes [bytes (Dfault:No Limit)]\n"
        )
    }
    args
}

fn read_port() -> u16 {
    let args = validate_args();
    let arg_number = ArgumentsOrder::Port as usize;
    let port = (&args[arg_number])
        .parse()
        .expect(format!("Specified Port {} is not a number", &args[arg_number]).as_str());
    port
}

fn read_args() -> CommandLineArguments {
    let args = validate_args();
    let arg_number = ArgumentsOrder::RateChunk as usize;
    let rate_chunk = (&args[arg_number])
        .parse()
        .expect(format!("Specified Rate Chunk {} is not a number", &args[arg_number]).as_str());
    let arg_number = ArgumentsOrder::RateTime as usize;
    let rate_time = (&args[arg_number])
        .parse()
        .expect(format!("Specified Rate Time {} is not a number", &args[arg_number]).as_str());
    let arg_number = ArgumentsOrder::Filename as usize;
    let filename = if args.len() > arg_number {
        &args[arg_number]
    } else {
        &create_file()
    };
    let arg_number = ArgumentsOrder::Repeat as usize;
    let repeat = if args.len() > arg_number {
        if (&args[arg_number]).to_lowercase() == "true" {
            true
        } else if &args[arg_number].to_lowercase() == "false" {
            false
        } else {
            panic!("Specified repeat {} isn't true or false", &args[arg_number]);
        }
    } else {
        false
    };
    let arg_number = ArgumentsOrder::MaxBytes as usize;
    let max_bytes =
        if args.len() > arg_number {
            Some((&args[arg_number]).parse().expect(
                format!("Specified Rate Chunk {} is not a number", &args[arg_number]).as_str(),
            ))
        } else {
            None
        };
    let content =
        fs::read_to_string(filename).expect(format!("Error Reading the file: {filename}").as_str());

    CommandLineArguments {
        content,
        rate_chunk,
        rate_time,
        repeat,
        max_bytes,
    }
}

fn write_to_stream(mut stream: TcpStream) {
    let args = read_args();
    let thread_id = thread::current().id();
    let up_limit = if args.max_bytes.is_some_and(|n| n < args.content.len()) {
        args.max_bytes.unwrap()
    } else {
        args.content.len()
    };
    loop {
        let mut pos = 0;
        while pos + args.rate_chunk < up_limit {
            let progress = 100.0 * (pos as f64 / up_limit as f64);
            println!("Sending data to client {thread_id:?} - Progress: {progress:.3}%");
            match stream.write(args.content[pos..pos + args.rate_chunk].as_bytes()) {
                Ok(_) => stream.flush().expect("Error Flushing"),
                Err(_) => println!(
                    "Writing {} to stream failed",
                    &args.content[pos..pos + args.rate_chunk]
                ),
            }
            pos += args.rate_chunk;
            thread::sleep(Duration::from_millis(args.rate_time));
        }
        match stream.write(args.content[pos..].as_bytes()) {
            Ok(_) => stream.flush().expect("Error Flushing"),
            Err(_) => println!("Writing {} to stream failed", &args.content[pos..]),
        }
        if args.repeat == false {
            break;
        }
    }
    println!("Data Transmition to Client {thread_id:?} is Finished");
}
