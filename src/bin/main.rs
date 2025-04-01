use std::{
    env, fs,
    io::Write,
    net::{TcpListener, TcpStream},
    sync::{Arc, RwLock},
    thread,
    time::Duration,
    usize,
};

use threadpool::ThreadPool;

fn main() {
    let args: Vec<String> = env::args().collect();
    let args = Arc::new(RwLock::new(read_args(args)));
    let pool = ThreadPool::new(4);
    let connection_id = Arc::new(RwLock::new(0));
    let port = (*args.read().unwrap()).port;
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let connection_id = Arc::clone(&connection_id);
        let args = Arc::clone(&args);
        pool.execute(move || {
            {
                (*connection_id.write().unwrap()) += 1;
            }
            write_to_stream(
                stream,
                connection_id.read().unwrap().clone(),
                args.read().unwrap().clone(),
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
    max_bytes: Option<usize>,
}

fn read_args(args: Vec<String>) -> CommandLineArguments {
    const MIN_ARG_COUNT: usize = 4;
    const MAX_ARG_COUNT: usize = 6;
    if args.len() < MIN_ARG_COUNT + 1 || args.len() > MAX_ARG_COUNT + 1 {
        panic!("Should have at least {MIN_ARG_COUNT} arguments and most {MAX_ARG_COUNT} arguments:\n
                filename [path] rate_chunk [bytes] rate_time [ms] port [0-65535] loop [true or false (Default: false)] max_bytes [bytes (Dfault:No Limit)]")
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
    let max_bytes = if args.len() == 7 {
        Some(
            (&args[6])
                .parse()
                .expect(format!("Specified Rate Chunk {} is not a number", &args[6]).as_str()),
        )
    } else {
        None
    };
    let content =
        fs::read_to_string(filename).expect(format!("Error Reading the file: {filename}").as_str());

    CommandLineArguments {
        content,
        rate_chunk,
        rate_time,
        port,
        repeat,
        max_bytes,
    }
}

fn write_to_stream(mut stream: TcpStream, connection_id: u32, args: CommandLineArguments) {
    loop {
        let mut pos = 0;
        let up_limit = if args.max_bytes.is_some_and(|n| n < args.content.len()) {
            args.max_bytes.unwrap()
        } else {
            args.content.len()
        };
        while pos + args.rate_chunk < up_limit {
            let progress = 100.0 * (pos as f64 / up_limit as f64);
            println!("Sending data to client #{connection_id} - Progress: {progress:.3}%");
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
