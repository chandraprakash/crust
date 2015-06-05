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


use cbor::CborTagEncode;
use rustc_serialize::{Decodable, Decoder, Encodable, Encoder};
use std::net::{SocketAddr, TcpStream, TcpListener, ToSocketAddrs};
use std::collections::BTreeMap;
//use sodiumoxide::crypto;
use transport::Endpoint;
//use std::fs::File;
//use std::io::prelude::*;
//use std::path;
//use std::env;
use std::fmt::Display;
//use std::io;
use rustc_serialize::json::{self, ToJson, Json};

// JSON value representation
impl ToJson for Endpoint {
    fn to_json(&self) -> Json {
        match *self {
            Endpoint::Tcp(socket_addr) => {
                Json::String(socket_addr.to_string())
            },
        }
    }
}

//#[derive(PartialEq, Debug, RustcDecodable, RustcEncodable)]
pub struct Contact {
    endpoint: Endpoint,
}

impl Contact {
    pub fn new(endpoint: Endpoint) -> Contact {
        Contact {
            endpoint: endpoint,
        }
    }
}

// Specify encoding method manually
impl ToJson for Contact {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        // All standard types implement `to_json()`, so use it
        d.insert("endpoint".to_string(), self.endpoint.to_json());
        Json::Object(d)
    }
}




// impl Encodable for Contact {
//     fn encode<E: Encoder>(&self, e: &mut E)->Result<(), E::Error> {
//         CborTagEncode::new(5483_400, &(&self.endpoint)).encode(e)
//     }
// }

// impl Decodable for Contact {
//     fn decode<D: Decoder>(d: &mut D)->Result<Contact, D::Error> {
//         let _ = try!(d.read_u64());
//         let endpoint : Endpoint = try!(Decodable::decode(d));
//         Ok(Contact::new(endpoint))
//     }
//}

impl Clone for Contact {
    fn clone(&self) -> Contact {
        Contact {
            endpoint: self.endpoint.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use bootstrap_handler::{Contact};
    use std::net;
    use std::net::SocketAddr;
//    use sodiumoxide;
    use cbor;
    use rand;
    use transport;
    use rustc_serialize::json::{self, ToJson, Json};

    #[test]
    fn serialisation() {
        let addr = net::SocketAddrV4::new(net::Ipv4Addr::new(1,2,3,4), 8080);
        let ep = transport::Endpoint::Tcp(SocketAddr::V4(addr));
        let contact  = Contact::new(transport::Endpoint::Tcp(SocketAddr::V4(addr)));


        let json_obj: Json = contact.to_json();
        let json_str: String = json_obj.to_string();
        println!("{:?}", json_str);

        // let contact_before = Contact::new(transport::Endpoint::Tcp(SocketAddr::V4(addr)));

        // let mut e = cbor::Encoder::from_memory();
        // e.encode(&[&contact_before]).unwrap();

        // let mut d = cbor::Decoder::from_bytes(e.as_bytes());
        // let contact_after: Contact = d.decode().next().unwrap().unwrap();
        // assert_eq!(contact_before, contact_after);
    }


//#[test]
// fn bla2() {
//     let mut random_addr_0 = Vec::with_capacity(4);
//     random_addr_0.push(rand::random::<u8>());
//     random_addr_0.push(rand::random::<u8>());
//     random_addr_0.push(rand::random::<u8>());
//     random_addr_0.push(rand::random::<u8>());

//     let port_0: u16 = rand::random::<u16>();
//     let addr_0 = net::SocketAddrV4::new(net::Ipv4Addr::new(random_addr_0[0], random_addr_0[1], random_addr_0[2], random_addr_0[3]), port_0);
//     println!("{:?}", addr_0);
//     let new_contact = Contact::new(Endpoint::Tcp(SocketAddr::V4(addr_0)));

//     // Serialize using `json::encode`
//     let encoded = json::encode(&new_contact).unwrap();

//     println!("{:?}", encoded);

//     // // Deserialize using `json::decode`
//     // let decoded: TestStruct = json::decode(&encoded).unwrap();
//     // println!("{:?}", decoded.data_int);
// }

}
