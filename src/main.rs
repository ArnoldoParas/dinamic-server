use std::{
    collections::HashMap, 
    io::{prelude::*, BufReader}, 
    net::{ TcpListener, TcpStream}, 
    sync::{Arc, Mutex}, 
    thread, time::Duration
};
// use tungstenite::stream;
use uuid::Uuid;

fn main() {
    let current_ip = Arc::new(Mutex::new(String::new()));
    let current_ip_clone = current_ip.clone();
    thread::spawn(|| {
        host(current_ip_clone);
    });
    
    loop{}
}

fn tcp_listener_thread(termination_signal: Arc<Mutex<bool>>, ip: Arc<Mutex<String>>) {
    let ip_clone = ip.clone();
    let mut ip_locked = ip_clone.lock().unwrap();
    if ip_locked.is_empty() {
        *ip_locked = "192.168.100.31:3012".to_string();
    }

    let listener = TcpListener::bind(&*ip_locked).unwrap();
    println!("listening on {}", *ip_locked);

    let mut hosts: HashMap<String, String> = HashMap::new();
    let switch = Arc::new(Mutex::new(false));

    let switch_clone = switch.clone();
    let termination_signal_clone = termination_signal.clone();

    thread::spawn(move ||{
        clk(switch_clone, termination_signal_clone);
    });

    for stream in listener.incoming() {
        {
            let signal = termination_signal.lock().expect("Fallo en checar la señal");
            if *signal {
                break;
            }
        }
        
        let stream = stream.expect("Fallo en inicial el strea?");
        let lock = switch.lock().expect("Error?");
        if dbg!(*lock){
          switch_connection(stream, &mut hosts);
          continue; 
        };
        handle_conecction(stream, &mut hosts);
    }
    thread::spawn(|| {
        host(ip);
    });
}

fn handle_conecction(mut stream: TcpStream, hosts: &mut HashMap<String, String>) {
    let buf_reader = BufReader::new(&mut stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("Request: {:#?}", http_request);
    
    let response;
    if http_request[0] == "Id: none" {
        let id = String::from(Uuid::new_v4());
        response = format!("Unauthorized\nNone\n{}", id);
        hosts.insert(
            id, 
            stream
                .peer_addr()
                .unwrap()
                .ip()
                .to_string()
        );
    } else {
        if hosts.get(&http_request[0]) != None {
            response = format!("OK\nNone\nUr id is: {}", http_request[0]);
            println!("----------\nhost ip: {}\n----------\n{:?}",stream.peer_addr().unwrap(), hosts);
        } else {
            response = format!("OK\nNone\nUr id is: {}", http_request[0]);
            println!("----------\nhost ip: {}\n----------\n{:?}",hosts.get(&http_request[0]).unwrap(), hosts);
        }
    }
    stream.write_all(response.as_bytes()).unwrap();
}

fn clk(sw: Arc<Mutex<bool>>, termination_signal: Arc<Mutex<bool>>) {
    loop {
        thread::sleep(Duration::from_secs(5));
        { // If server switch
            let mut lock = sw.lock().unwrap();
            *lock = true;
        }
        thread::sleep(Duration::from_secs(4));
        let mut signal = termination_signal.lock().unwrap();
        *signal = true;
        let mut stream = TcpStream::connect("192.168.100.31:3012").unwrap();
        stream.write_all("end".as_bytes()).unwrap();
        break;
    }
}

fn switch_connection(mut stream: TcpStream, hosts: &mut HashMap<String, String>) {
    let buf_reader = BufReader::new(&mut stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("Request: {:#?}", http_request);

    if stream.peer_addr().unwrap().ip().to_string() == hosts.get(&http_request[0]).unwrap().to_owned() {
        let ip = stream.peer_addr().unwrap().ip().to_string();
        let response = format!("OK\n{}", ip);
        
        stream.write_all(response.as_bytes()).unwrap();

    } else {
        let response;
        if http_request[0] == "Id: none" {
            let id = String::from(Uuid::new_v4());
            response = format!("Unauthorized\n{}", id);
            hosts.insert(
                id, 
                stream
                    .peer_addr()
                    .unwrap()
                    .ip()
                    .to_string()
            );
        } else {
            if hosts.get(&http_request[0]) != None {
                response = format!("OK\nNone\nUr id is: {}", http_request[0]);
                println!("----------\nhost ip: {}\n----------\n{:?}",stream.peer_addr().unwrap(), hosts);
            } else {
                response = format!("OK\nNone\nUr id is: {}", http_request[0]);
                println!("----------\nhost ip: {}\n----------\n{:?}",hosts.get(&http_request[0]).unwrap(), hosts);
            }
        }
        stream.write_all(response.as_bytes()).unwrap();
    }
}

fn host(ip: Arc<Mutex<String>>) {
    let ip_clone = ip.clone();
    let mut ip_locked = ip_clone.lock().unwrap();
    if ip_locked.is_empty() {
        *ip_locked = "192.168.100.31:3012".to_string();
    }

    let mut response = String::from("Id: none\nHeader 1\nHeader 2\nBody");
    loop {
        let mut stream;
        match TcpStream::connect(&*ip_locked) {
            Ok(s) => stream = s,
            Err(_) => {
                thread::spawn(move ||{
                    let termination_signal = Arc::new(Mutex::new(false));
                    tcp_listener_thread(termination_signal, ip);
                    println!("FINALIZADO");
                });
                break;
            }
        };

        stream.write_all(response.as_bytes()).expect("fallo en enviar el error");
        stream.shutdown(std::net::Shutdown::Write).unwrap();

        let buf_reader = BufReader::new(&mut stream);
        let http_response: Vec<_> = buf_reader
            .lines()
            .map(|result| result.unwrap())
            .take_while(|line| !line.is_empty())
            .collect();

        if http_response[0] != "OK" {
            response = format!("{}\nHeader 1\nHeader 2\nBody", http_response[2]);
        } 
        if http_response[1] != "None" {
            *ip_locked = dbg!(format!("{}:3012",&http_response[1]));
            thread::spawn(move ||{
                println!("Response: {:#?}", http_response);
                println!("----------\nhost ip: {}\n----------",stream.peer_addr().unwrap());
                
                let termination_signal = Arc::new(Mutex::new(false));
                tcp_listener_thread(termination_signal, ip);
                println!("FINALIZADO");
            });
            break;
        }
        println!("Response: {:#?}", http_response);
        println!("----------\nhost ip: {}\n----------",stream.peer_addr().unwrap());
        thread::sleep(Duration::from_secs(1));
    }
}