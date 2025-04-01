use std::{
    env, fs,
    io::Write,
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
    usize,
};

use threadpool::ThreadPool;

fn main() {
    let args: Vec<String> = env::args().collect();
    let args = Arc::new(Mutex::new(read_args(args)));
    let pool = ThreadPool::new(4);
    let connection_id = Arc::new(Mutex::new(0));
    let port = (*args.lock().unwrap()).port;
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let connection_id = Arc::clone(&connection_id);
        let args = Arc::clone(&args);
        pool.execute(move || {
            (*connection_id.lock().unwrap()) += 1;
            write_to_stream(
                stream,
                connection_id.lock().unwrap().clone(),
                args.lock().unwrap().clone(),
            );
            println!("Data Transmition Finished");
        });
    }
}

#[derive(Clone)]
struct CommandLineArguments {
    content: String,
    rate_chunk: usize,
    rate_time: u64,
    port: u16,
    repeat: bool,
}

fn read_args(args: Vec<String>) -> CommandLineArguments {
    const MIN_ARG_COUNT: usize = 4;
    const MAX_ARG_COUNT: usize = 5;
    if args.len() < MIN_ARG_COUNT + 1 || args.len() > MAX_ARG_COUNT + 1 {
        panic!("Should have at least {MIN_ARG_COUNT} arguments and most {MAX_ARG_COUNT} arguments:\n
                filename [path] rate_chunk [bytes] rate_time [ms] port [0-65535] loop [true or false (Default: false)]")
    }
    let filename = &args[1];
    let rate_chunk = (&args[2])
        .parse()
        .expect(format!("Specified Rate Chunk {} is not a number", &args[2]).as_str());
    let rate_time = (&args[3])
        .parse()
        .expect(format!("Specified Rate Time {} is not a number", &args[3]).as_str());
    let port = (&args[4])
        .parse()
        .expect(format!("Specified Port {} is not a number", &args[4]).as_str());
    let repeat = if args.len() == 6 {
        if (&args[5]).to_lowercase() == "true" {
            true
        } else if &args[5].to_lowercase() == "false" {
            false
        } else {
            panic!("Specified loop {} isn't true or false", &args[4]);
        }
    } else {
        false
    };
    let content =
        fs::read_to_string(filename).expect(format!("Error Reading the file: {filename}").as_str());

    CommandLineArguments {
        content,
        rate_chunk,
        rate_time,
        port,
        repeat,
    }
}

fn write_to_stream(mut stream: TcpStream, connection_id: u32, args: CommandLineArguments) {
    loop {
        let mut pos = 0;
        while pos + args.rate_chunk < args.content.len() {
            println!("Sending data to client #{connection_id}");
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
        println!("Sending last chunk to client #{connection_id}");
        match stream.write(args.content[pos..].as_bytes()) {
            Ok(_) => stream.flush().expect("Error Flushing"),
            Err(_) => println!("Writing {} to stream failed", &args.content[pos..]),
        }
        if args.repeat == false {
            break;
        }
    }
}
