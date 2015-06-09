// Copyright 2015 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under (1) the MaidSafe.net Commercial License,
// version 1.0 or later, or (2) The General Public License (GPL), version 3, depending on which
// licence you accepted on initial access to the Software (the "Licences").
//
// By contributing code to the SAFE Network Software, or to this project generally, you agree to be
// bound by the terms of the MaidSafe Contributor Agreement, version 1.0.  This, along with the
// Licenses can be found in the root directory of this project at LICENSE, COPYING and CONTRIBUTOR.
//
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.
//
// Please review the Licences for the specific language governing permissions and limitations
// relating to use of the SAFE Network Software.

use std::net::{SocketAddr, TcpStream, TcpListener, ToSocketAddrs};
use tcp_connections;
use std::io;
use std::io::Write;
use std::io::Result as IoResult;
use std::error::Error;
use std::sync::mpsc;
use std::str;
use std::str::FromStr;
use beacon;
use cbor::CborTagEncode;
use rustc_serialize::{Decodable, Decoder, Encodable, Encoder};
use std::cmp::Ordering;
pub type Bytes = Vec<u8>;

fn array_to_vec(arr: &[u8]) -> Vec<u8> {
    let mut vector = Vec::new();
    for i in arr.iter() {
        vector.push(*i);
    }
    vector
}

/// Enum representing endpoint of supported protocols
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Endpoint {
    /// TCP endpoint
    Tcp(SocketAddr),
}

impl Endpoint {
    /// Creates a Tcp(SocketAddr)
    pub fn tcp<A: ToSocketAddrs>(addr: A) -> Endpoint {
        match addr.to_socket_addrs().unwrap().next() {
            Some(a) => Endpoint::Tcp(a),
            None => panic!("Failed to parse valid IP address"),
        }
    }
    /// Returns SocketAddr.
    pub fn get_address(&self) -> SocketAddr {
        match *self {
            Endpoint::Tcp(address) => address,
        }
    }
}



// impl Encodable for Endpoint {
//     fn encode<E: Encoder>(&self, e: &mut E)->Result<(), E::Error> {
//         let address = match *self {
//             Endpoint::Tcp(socket_addr) => socket_addr.to_string()
//         };
//         try!(e.emit_struct_field( "endpoint", 0usize, |e| address.encode(e)));
//         Ok(())
//     }
// }


// impl Decodable for Endpoint {
//     fn decode<D: Decoder>(d: &mut D)->Result<Endpoint, D::Error> {
//         d.read_struct("root", 0, |d| {
//                 d.read_struct_field("data", 0, |d| {
//                         let address = try!(d.read_struct_field("endpoint", 0usize, |d| {
//                                 let address_string: String = try!(Decodable::decode(d));
//                                 Ok(SocketAddr::from_str(&address_string).unwrap())
//                         }));
//                         return Ok(Endpoint::Tcp(address));
//                 })
//         })
//     }
// }

impl Encodable for Endpoint {
    fn encode<E: Encoder>(&self, e: &mut E)->Result<(), E::Error> {
        let address = match *self {
            Endpoint::Tcp(socket_addr) => socket_addr.to_string()
        };
        try!(address.encode(e));
        Ok(())
    }
}

impl Decodable for Endpoint {
    fn decode<D: Decoder>(d: &mut D)->Result<Endpoint, D::Error> {
        let decoded: String = try!(Decodable::decode(d));
        let address: SocketAddr = SocketAddr::from_str(&decoded).unwrap();
        Ok(Endpoint::Tcp(address))
    }
}

/// Enum representing port of supported protocols
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Port {
    /// TCP port
    Tcp(u16),
}

impl Port {
    /// Return the port
    pub fn get_port(&self) -> u16 {
        match *self {
            Port::Tcp(p) => p,
        }
    }
}

impl PartialOrd for Endpoint {
    fn partial_cmp(&self, other: &Endpoint) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Endpoint {
    fn cmp(&self, other: &Endpoint) -> Ordering {
        use Endpoint::Tcp;
        match *self {
            Tcp(ref a1) => match *other {
                Tcp(ref a2) => compare_ip_addrs(a1, a2)
            }
        }
    }
}

//--------------------------------------------------------------------
pub enum Sender {
    Tcp(tcp_connections::TcpWriter<Bytes>),
}

impl Sender {
    pub fn send(&mut self, bytes: &Bytes) -> IoResult<()> {
        match *self {
            Sender::Tcp(ref mut s) => {
                s.send(&bytes).map_err(|_| {
                // FIXME: This can be done better.
                io::Error::new(io::ErrorKind::NotConnected, "can't send")
            })
            },
        }
    }
}

//--------------------------------------------------------------------
pub enum Receiver {
    Tcp(tcp_connections::TcpReader<Bytes>),
}

impl Receiver {
    pub fn receive(&self) -> IoResult<Bytes> {
        match *self {
            Receiver::Tcp(ref r) => {
                match r.recv() {
                    Ok(data) => Ok(data),
                    Err(what) => {
                        return Err(io::Error::new(io::ErrorKind::NotConnected, what.description()));
                    },
                }
            }
        }
    }
}

//--------------------------------------------------------------------
pub enum Acceptor {
    // Channel receiver, TCP listener
    Tcp(mpsc::Receiver<(TcpStream, SocketAddr)>, TcpListener),
}

impl Acceptor {
    pub fn local_port(&self) -> Port {
        match *self {
            Acceptor::Tcp(_, ref listener) => Port::Tcp(listener.local_addr().unwrap().port()),
        }
    }
}

pub struct Transport {
    pub receiver: Receiver,
    pub sender: Sender,
    pub remote_endpoint: Endpoint,
}

// FIXME: There needs to be a way to break from this blocking command.
pub fn connect(remote_ep: Endpoint) -> IoResult<Transport> {
    match remote_ep {
        Endpoint::Tcp(ep) => {
            tcp_connections::connect_tcp(ep)
                .map(|(i, o)| {
                    Transport{ receiver: Receiver::Tcp(i),
                                         sender: Sender::Tcp(o),
                                         remote_endpoint: remote_ep,
                             }})
                .map_err(|e| {
                    let _ = writeln!(&mut io::stderr(), "NOTE: Transport connect {} failure due to {}", ep, e);
                    io::Error::new(io::ErrorKind::NotConnected, e.description())
                })
        }
    }
}

pub fn new_acceptor(port: &Port) -> IoResult<Acceptor> {
    match *port {
        Port::Tcp(ref port) => {
            let (receiver, listener) = try!(tcp_connections::listen(*port));
            Ok(Acceptor::Tcp(receiver, listener))
        }
    }
}

// FIXME: There needs to be a way to break from this blocking command.
// (Though this seems to be impossible with the current Rust TCP API).
pub fn accept(acceptor: &Acceptor) -> IoResult<Transport> {
    match *acceptor {
        Acceptor::Tcp(ref rx_channel, _) => {
            let rx = rx_channel.recv();
            let (stream, remote_endpoint) = try!(rx
                .map_err(|e| io::Error::new(io::ErrorKind::NotConnected, e.description())));

            let (i, o) = try!(tcp_connections::upgrade_tcp(stream));

            Ok(Transport{ receiver: Receiver::Tcp(i),
                          sender: Sender::Tcp(o),
                          remote_endpoint: Endpoint::Tcp(remote_endpoint),
                        })
        }
    }
}

fn compare_ip_addrs(a1: &SocketAddr, a2: &SocketAddr) -> Ordering {
    use std::net::SocketAddr::{V4,V6};
    match *a1 {
        V4(ref a1) => match *a2 {
            V4(ref a2) => (a1.ip(), a1.port()).cmp(&(a2.ip(), a2.port())),
            V6(_) => Ordering::Less,
        },
        V6(ref a1) => match *a2 {
            V4(_) => Ordering::Greater,
            V6(ref a2) => (a1.ip(), a1.port(), a1.flowinfo(), a1.scope_id())
                          .cmp(&(a2.ip(), a2.port(), a2.flowinfo(), a2.scope_id())),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6, Ipv4Addr, Ipv6Addr};
    use rand;
    use std::net;
    use cbor;
    use rustc_serialize::json;

    fn v4(a: u8, b: u8, c: u8, d: u8, e: u16) -> Endpoint {
        Endpoint::Tcp(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(a,b,c,d),e)))
    }

    fn v6(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16, i: u16, j: u32, k: u32) -> Endpoint {
        Endpoint::Tcp(SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::new(a,b,c,d,e,f,g,h),i, j, k)))
    }

    fn test(smaller: Endpoint, bigger: Endpoint) {
        assert!(smaller < bigger);
        assert!(bigger > smaller);
        assert!(smaller != bigger);
    }

#[test]
    fn test_ord() {
        test(v4(1,2,3,4,5), v4(2,2,3,4,5));
        test(v4(1,2,3,4,5), v4(1,3,3,4,5));
        test(v4(1,2,3,4,5), v4(1,2,4,4,5));
        test(v4(1,2,3,4,5), v4(1,2,3,5,5));
        test(v4(1,2,3,4,5), v4(1,2,3,4,6));

        test(v4(1,2,3,4,5), v6(0,0,0,0,0,0,0,0,0,0,0));
        test(v4(1,2,3,4,5), v6(1,2,3,4,5,6,7,8,9,10,11));
        test(v4(1,2,3,4,5), v6(2,3,4,5,6,7,8,9,10,11,12));

        test(v6(0,0,0,0,0,0,0,0,0,0,0), v6(1,0,0,0,0,0,0,0,0,0,0));
        test(v6(0,0,0,0,0,0,0,0,0,0,0), v6(0,1,0,0,0,0,0,0,0,0,0));
        test(v6(0,0,0,0,0,0,0,0,0,0,0), v6(0,0,1,0,0,0,0,0,0,0,0));
        test(v6(0,0,0,0,0,0,0,0,0,0,0), v6(0,0,0,1,0,0,0,0,0,0,0));
        test(v6(0,0,0,0,0,0,0,0,0,0,0), v6(0,0,0,0,1,0,0,0,0,0,0));
        test(v6(0,0,0,0,0,0,0,0,0,0,0), v6(0,0,0,0,0,1,0,0,0,0,0));
        test(v6(0,0,0,0,0,0,0,0,0,0,0), v6(0,0,0,0,0,0,1,0,0,0,0));
        test(v6(0,0,0,0,0,0,0,0,0,0,0), v6(0,0,0,0,0,0,0,1,0,0,0));
        test(v6(0,0,0,0,0,0,0,0,0,0,0), v6(0,0,0,0,0,0,0,0,1,0,0));
        test(v6(0,0,0,0,0,0,0,0,0,0,0), v6(0,0,0,0,0,0,0,0,0,1,0));
        test(v6(0,0,0,0,0,0,0,0,0,0,0), v6(0,0,0,0,0,0,0,0,0,0,1));

        test(v6(1,2,3,4,5,6,7,8,9,10,11), v6(2,2,3,4,5,6,7,8,9,10,11));
        test(v6(1,2,3,4,5,6,7,8,9,10,11), v6(1,3,3,4,5,6,7,8,9,10,11));
        test(v6(1,2,3,4,5,6,7,8,9,10,11), v6(1,2,4,4,5,6,7,8,9,10,11));
        test(v6(1,2,3,4,5,6,7,8,9,10,11), v6(1,2,3,5,5,6,7,8,9,10,11));
        test(v6(1,2,3,4,5,6,7,8,9,10,11), v6(1,2,3,4,6,6,7,8,9,10,11));
        test(v6(1,2,3,4,5,6,7,8,9,10,11), v6(1,2,3,4,5,7,7,8,9,10,11));
        test(v6(1,2,3,4,5,6,7,8,9,10,11), v6(1,2,3,4,5,6,8,8,9,10,11));
        test(v6(1,2,3,4,5,6,7,8,9,10,11), v6(1,2,3,4,5,6,7,9,9,10,11));
        test(v6(1,2,3,4,5,6,7,8,9,10,11), v6(1,2,3,4,5,6,7,8,10,10,11));
        test(v6(1,2,3,4,5,6,7,8,9,10,11), v6(1,2,3,4,5,6,7,8,9,11,11));
        test(v6(1,2,3,4,5,6,7,8,9,10,11), v6(1,2,3,4,5,6,7,8,9,10,12));
    }

#[test]
fn endpoint_serialisation() {
    let mut random_addr_0 = Vec::with_capacity(4);
    random_addr_0.push(rand::random::<u8>());
    random_addr_0.push(rand::random::<u8>());
    random_addr_0.push(rand::random::<u8>());
    random_addr_0.push(rand::random::<u8>());
    let port_0: u16 = rand::random::<u16>();
    let addr_0 = net::SocketAddrV4::new(net::Ipv4Addr::new(random_addr_0[0], random_addr_0[1], random_addr_0[2], random_addr_0[3]), port_0);
    let ep = Endpoint::Tcp(SocketAddr::V4(addr_0));

    // Serialize using `json::encode`
    let encoded = json::encode(&ep).unwrap();

    println!("encoded: {:?}", encoded);

    // Deserialize using `json::decode`
    let decoded: Endpoint = json::decode(&encoded).unwrap();
    assert_eq!(decoded, ep);
}
}
