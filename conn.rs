use std::{fmt, str, uint, default};
use std::io::{Stream, Reader, File, IoResult, Seek,
              SeekCur, EndOfFile, BufReader, MemWriter};
use std::io::net::ip::{SocketAddr, Ipv4Addr, Ipv6Addr};
use std::io::net::tcp::{TcpStream};
use std::io::net::unix::{UnixStream};
use super::consts;
use super::scramble::{scramble};
use super::io::{MyReader, MyWriter};
use super::error::{MyError, MyIoError, MySqlError, MyStrError};

pub type MyResult<T> = Result<T, MyError>;

/***
 *     .d88888b.  888      8888888b.                    888               888    
 *    d88P" "Y88b 888      888   Y88b                   888               888    
 *    888     888 888      888    888                   888               888    
 *    888     888 888  888 888   d88P  8888b.   .d8888b 888  888  .d88b.  888888 
 *    888     888 888 .88P 8888888P"      "88b d88P"    888 .88P d8P  Y8b 888    
 *    888     888 888888K  888        .d888888 888      888888K  88888888 888    
 *    Y88b. .d88P 888 "88b 888        888  888 Y88b.    888 "88b Y8b.     Y88b.  
 *     "Y88888P"  888  888 888        "Y888888  "Y8888P 888  888  "Y8888   "Y888 
 *                                                                               
 *                                                                               
 *                                                                               
 */

pub struct OkPacket {
    affected_rows: u64,
    last_insert_id: u64,
    status_flags: u16,
    warnings: u16,
    info: Vec<u8>
}

impl OkPacket {
    #[inline]
    fn from_payload(pld: &[u8]) -> IoResult<OkPacket> {
        let mut reader = BufReader::new(pld);
        try!(reader.seek(1, SeekCur));
        Ok(OkPacket{
            affected_rows: try!(reader.read_lenenc_int()),
            last_insert_id: try!(reader.read_lenenc_int()),
            status_flags: try!(reader.read_le_u16()),
            warnings: try!(reader.read_le_u16()),
            info: try!(reader.read_to_end())
        })
    }
}

/***
 *    8888888888                 8888888b.                    888               888    
 *    888                        888   Y88b                   888               888    
 *    888                        888    888                   888               888    
 *    8888888    888d888 888d888 888   d88P  8888b.   .d8888b 888  888  .d88b.  888888 
 *    888        888P"   888P"   8888888P"      "88b d88P"    888 .88P d8P  Y8b 888    
 *    888        888     888     888        .d888888 888      888888K  88888888 888    
 *    888        888     888     888        888  888 Y88b.    888 "88b Y8b.     Y88b.  
 *    8888888888 888     888     888        "Y888888  "Y8888P 888  888  "Y8888   "Y888 
 *                                                                                     
 *                                                                                     
 *                                                                                     
 */

pub struct ErrPacket {
    sql_state: Vec<u8>,
    error_message: Vec<u8>,
    error_code: u16
}

impl ErrPacket {
    #[inline]
    fn from_payload(pld: &[u8]) -> IoResult<ErrPacket> {
        let mut reader = BufReader::new(pld);
        try!(reader.seek(1, SeekCur));
        let error_code = try!(reader.read_le_u16());
        try!(reader.seek(1, SeekCur));
        Ok(ErrPacket{
            error_code: error_code,
            sql_state: try!(reader.read_exact(5)),
            error_message: try!(reader.read_to_end())
        })
    }
}

impl fmt::Show for ErrPacket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f.buf,
               "ERROR {:u} ({:s}): {:s}",
               self.error_code,
               str::from_utf8(self.sql_state.as_slice()).unwrap(),
               str::from_utf8(self.error_message.as_slice()).unwrap())
    }
}

/***
 *    8888888888  .d88888b.  8888888888 8888888b.                    888               888    
 *    888        d88P" "Y88b 888        888   Y88b                   888               888    
 *    888        888     888 888        888    888                   888               888    
 *    8888888    888     888 8888888    888   d88P  8888b.   .d8888b 888  888  .d88b.  888888 
 *    888        888     888 888        8888888P"      "88b d88P"    888 .88P d8P  Y8b 888    
 *    888        888     888 888        888        .d888888 888      888888K  88888888 888    
 *    888        Y88b. .d88P 888        888        888  888 Y88b.    888 "88b Y8b.     Y88b.  
 *    8888888888  "Y88888P"  888        888        "Y888888  "Y8888P 888  888  "Y8888   "Y888 
 *                                                                                            
 *                                                                                            
 *                                                                                            
 */

pub struct EOFPacket {
    warnings: u16,
    status_flags: u16
}

impl EOFPacket {
    #[inline]
    fn from_payload(pld: &[u8]) -> IoResult<EOFPacket> {
        let mut reader = BufReader::new(pld);
        try!(reader.seek(1, SeekCur));
        Ok(EOFPacket{
            warnings: try!(reader.read_le_u16()),
            status_flags: try!(reader.read_le_u16())
        })
    }
}

/***
 *    888    888                        888          888               888               8888888b.                    888               888    
 *    888    888                        888          888               888               888   Y88b                   888               888    
 *    888    888                        888          888               888               888    888                   888               888    
 *    8888888888  8888b.  88888b.   .d88888 .d8888b  88888b.   8888b.  888  888  .d88b.  888   d88P  8888b.   .d8888b 888  888  .d88b.  888888 
 *    888    888     "88b 888 "88b d88" 888 88K      888 "88b     "88b 888 .88P d8P  Y8b 8888888P"      "88b d88P"    888 .88P d8P  Y8b 888    
 *    888    888 .d888888 888  888 888  888 "Y8888b. 888  888 .d888888 888888K  88888888 888        .d888888 888      888888K  88888888 888    
 *    888    888 888  888 888  888 Y88b 888      X88 888  888 888  888 888 "88b Y8b.     888        888  888 Y88b.    888 "88b Y8b.     Y88b.  
 *    888    888 "Y888888 888  888  "Y88888  88888P' 888  888 "Y888888 888  888  "Y8888  888        "Y888888  "Y8888P 888  888  "Y8888   "Y888 
 *                                                                                                                                             
 *                                                                                                                                             
 *                                                                                                                                             
 */

pub struct HandshakePacket {
    auth_plugin_data: Vec<u8>,
    auth_plugin_name: Vec<u8>,
    connection_id: u32,
    capability_flags: u32,
    status_flags: u16,
    protocol_version: u8,
    character_set: u8,
}

impl HandshakePacket {
    #[inline]
    fn from_payload(pld: &[u8]) -> IoResult<HandshakePacket> {
        let mut length_of_auth_plugin_data = 0i16;
        let mut auth_plugin_data: Vec<u8> = Vec::with_capacity(32);
        let mut auth_plugin_name: Vec<u8> = Vec::with_capacity(32);
        let mut character_set = 0u8;
        let mut status_flags = 0u16;
        let payload_len = pld.len();
        let mut reader = BufReader::new(pld);
        let protocol_version = try!(reader.read_u8());
        // skip server version
        while try!(reader.read_u8()) != 0u8 {}
        let connection_id = try!(reader.read_le_u32());
        try!(reader.push_exact(&mut auth_plugin_data, 8));
        // skip filler
        try!(reader.seek(1, SeekCur));
        let mut capability_flags = try!(reader.read_le_u16()) as u32;
        if try!(reader.tell()) != payload_len as u64 {
            character_set = try!(reader.read_u8());
            status_flags = try!(reader.read_le_u16());
            capability_flags |= (try!(reader.read_le_u16()) as u32) << 16;
            if (capability_flags & consts::CLIENT_PLUGIN_AUTH) > 0 {
                length_of_auth_plugin_data = try!(reader.read_u8()) as i16;
            } else {
                try!(reader.seek(1, SeekCur));
            }
            try!(reader.seek(10, SeekCur));
            if (capability_flags & consts::CLIENT_SECURE_CONNECTION) > 0 {
                let mut len = length_of_auth_plugin_data - 8i16;
                len = if len > 13i16 { len } else { 13i16 };
                try!(reader.push_exact(&mut auth_plugin_data, len as uint));
                if *auth_plugin_data.get(auth_plugin_data.len() - 1) == 0u8 {
                    auth_plugin_data.pop();
                }
            }
            if (capability_flags & consts::CLIENT_PLUGIN_AUTH) > 0 {
                auth_plugin_name = try!(reader.read_to_end());
                if *auth_plugin_name.get(auth_plugin_name.len() - 1) == 0u8 {
                    auth_plugin_name.pop();
                }
            }
        }
        Ok(HandshakePacket{protocol_version: protocol_version, connection_id: connection_id,
                         auth_plugin_data: auth_plugin_data,
                         capability_flags: capability_flags, character_set: character_set,
                         status_flags: status_flags, auth_plugin_name: auth_plugin_name})
    }
}

/***
 *     .d8888b.  888                  888    
 *    d88P  Y88b 888                  888    
 *    Y88b.      888                  888    
 *     "Y888b.   888888 88888b.d88b.  888888 
 *        "Y88b. 888    888 "888 "88b 888    
 *          "888 888    888  888  888 888    
 *    Y88b  d88P Y88b.  888  888  888 Y88b.  
 *     "Y8888P"   "Y888 888  888  888  "Y888 
 *                                           
 *                                           
 *                                           
 */

pub struct Stmt {
    params: Option<Vec<Column>>,
    columns: Option<Vec<Column>>,
    statement_id: u32,
    num_columns: u16,
    num_params: u16,
    warning_count: u16,
}

impl Stmt {
    #[inline]
    fn from_payload(pld: &[u8]) -> IoResult<Stmt> {
        let mut reader = BufReader::new(pld);
        try!(reader.seek(1, SeekCur));
        let statement_id = try!(reader.read_le_u32());
        let num_columns = try!(reader.read_le_u16());
        let num_params = try!(reader.read_le_u16());
        let warning_count = try!(reader.read_le_u16());
        Ok(Stmt{statement_id: statement_id,
              num_columns: num_columns,
              num_params: num_params,
              warning_count: warning_count,
              params: None,
              columns: None})
    }
}

/***
 *     .d8888b.           888                                 
 *    d88P  Y88b          888                                 
 *    888    888          888                                 
 *    888         .d88b.  888 888  888 88888b.d88b.  88888b.  
 *    888        d88""88b 888 888  888 888 "888 "88b 888 "88b 
 *    888    888 888  888 888 888  888 888  888  888 888  888 
 *    Y88b  d88P Y88..88P 888 Y88b 888 888  888  888 888  888 
 *     "Y8888P"   "Y88P"  888  "Y88888 888  888  888 888  888 
 *                                                            
 *                                                            
 *                                                            
 */

#[deriving(Clone)]
pub struct Column {
    catalog: Vec<u8>,
    schema: Vec<u8>,
    table: Vec<u8>,
    org_table: Vec<u8>,
    name: Vec<u8>,
    org_name: Vec<u8>,
    default_values: Vec<u8>,
    column_length: u32,
    character_set: u16,
    flags: u16,
    column_type: u8,
    decimals: u8
}

impl Column {
    #[inline]
    fn from_payload(command: u8, pld: &[u8]) -> IoResult<Column> {
        let mut reader = BufReader::new(pld);
        let catalog = try!(reader.read_lenenc_bytes());
        let schema = try!(reader.read_lenenc_bytes());
        let table = try!(reader.read_lenenc_bytes());
        let org_table = try!(reader.read_lenenc_bytes());
        let name = try!(reader.read_lenenc_bytes());
        let org_name = try!(reader.read_lenenc_bytes());
        // skip next length
        let _ = reader.read_lenenc_int();
        let character_set = try!(reader.read_le_u16());
        let column_length = try!(reader.read_le_u32());
        let column_type = try!(reader.read_u8());
        let flags = try!(reader.read_le_u16());
        let decimals = try!(reader.read_u8());
        // skip filler
        try!(reader.seek(2, SeekCur));
        let mut default_values = Vec::with_capacity(0);
        if command == consts::COM_FIELD_LIST {
            let len = try!(reader.read_lenenc_int());
            default_values = try!(reader.read_exact(len as uint));
        }
        Ok(Column{catalog: catalog,
                schema: schema,
                table: table,
                org_table: org_table,
                name: name,
                org_name: org_name,
                character_set: character_set,
                column_length: column_length,
                column_type: column_type,
                flags: flags,
                decimals: decimals,
                default_values: default_values})
    }
}

/***
 *    888     888          888                   
 *    888     888          888                   
 *    888     888          888                   
 *    Y88b   d88P  8888b.  888 888  888  .d88b.  
 *     Y88b d88P      "88b 888 888  888 d8P  Y8b 
 *      Y88o88P   .d888888 888 888  888 88888888 
 *       Y888P    888  888 888 Y88b 888 Y8b.     
 *        Y8P     "Y888888 888  "Y88888  "Y8888  
 *                                               
 *                                               
 *                                               
 */

#[deriving(Clone, Eq, Ord)]
pub enum Value {
    NULL,
    Bytes(Vec<u8>),
    Int(i64),
    UInt(u64),
    Float(f64),
    // year, month, day, hour, minutes, seconds, micro seconds
    Date(u16, u8, u8, u8, u8, u8, u32),
    // is negative, days, hours, minutes, seconds, micro seconds
    Time(bool, u32, u8, u8, u8, u32)
}

impl Value {
    /// Get correct string representation of a mysql value
    pub fn into_str(&self) -> ~str {
        match *self {
            NULL => ~"NULL",
            Bytes(ref x) => {
                match str::from_utf8(x.as_slice()) {
                    Some(s) => {
                        let replaced = s.replace("'", "\'");
                        format!("'{:s}'", replaced)
                    },
                    None => {
                        let mut s = ~"0x";
                        for c in x.iter() {
                            s = s.append(format!("{:02X}", *c));
                        }
                        s
                    }
                }
            },
            Int(x) => format!("{:d}", x),
            UInt(x) => format!("{:u}", x),
            Float(x) => format!("{:f}", x),
            Date(0, 0, 0, 0, 0, 0, 0) => ~"''",
            Date(y, m, d, 0, 0, 0, 0) => format!("'{:04u}-{:02u}-{:02u}'", y, m, d),
            Date(y, m, d, h, i, s, 0) => format!("'{:04u}-{:02u}-{:02u} {:02u}:{:02u}:{:02u}'", y, m, d, h, i, s),
            Date(y, m, d, h, i, s, u) => format!("'{:04u}-{:02u}-{:02u} {:02u}:{:02u}:{:02u}.{:06u}'", y, m, d, h, i, s, u),
            Time(_, 0, 0, 0, 0, 0) => ~"''",
            Time(neg, d, h, i, s, 0) => {
                if neg {
                    format!("'-{:u} {:03u}:{:02u}:{:02u}'", d, h, i, s)
                } else {
                    format!("'{:u} {:03u}:{:02u}:{:02u}'", d, h, i, s)
                }
            },
            Time(neg, d, h, i, s, u) => {
                if neg {
                    format!("'-{:u} {:03u}:{:02u}:{:02u}.{:06u}'", d, h, i, s, u)
                } else {
                    format!("'{:u} {:03u}:{:02u}:{:02u}.{:06u}'", d, h, i, s, u)
                }
            }
        }
    }
    pub fn is_bytes(&self) -> bool {
        match *self {
            Bytes(..) => true,
            _ => false
        }
    }
    pub fn bytes_ref<'a>(&'a self) -> &'a [u8] {
        match *self {
            Bytes(ref x) => x.as_slice(),
            _ => fail!("Called `Value::bytes_ref()` on non `Bytes` value")
        }
    }
    pub fn unwrap_bytes(self) -> Vec<u8> {
        match self {
            Bytes(x) => x,
            _ => fail!("Called `Value::unwrap_bytes()` on non `Bytes` value")
        }
    }
    pub fn unwrap_bytes_or(self, y: Vec<u8>) -> Vec<u8> {
        match self {
            Bytes(x) => x,
            _ => y
        }
    }
    pub fn is_int(&self) -> bool {
        match *self {
            Int(..) => true,
            _ => false
        }
    }
    pub fn get_int(&self) -> i64 {
        match *self {
            Int(x) => x,
            _ => fail!("Called `Value::get_int()` on non `Int` value")
        }
    }
    pub fn get_int_or(&self, y: i64) -> i64 {
        match *self {
            Int(x) => x,
            _ => y
        }
    }
    pub fn is_uint(&self) -> bool {
        match *self {
            UInt(..) => true,
            _ => false
        }
    }
    pub fn get_uint(&self) -> u64 {
        match *self {
            UInt(x) => x,
            _ => fail!("Called `Value::get_uint()` on non `UInt` value")
        }
    }
    pub fn get_uint_or(&self, y: u64) -> u64 {
        match *self {
            UInt(x) => x,
            _ => y
        }
    }
    pub fn is_float(&self) -> bool {
        match *self {
            Float(..) => true,
            _ => false
        }
    }
    pub fn get_float(&self) -> f64 {
        match *self {
            Float(x) => x,
            _ => fail!("Called `Value::get_float()` on non `Float` value")
        }
    }
    pub fn get_float_or(&self, y: f64) -> f64 {
        match *self {
            Float(x) => x,
            _ => y
        }
    }
    pub fn is_date(&self) -> bool {
        match *self {
            Date(..) => true,
            _ => false
        }
    }
    pub fn get_year(&self) -> u16 {
        match *self {
            Date(y, _, _, _, _, _, _) => y,
            _ => fail!("Called `Value::get_year()` on non `Date` value")
        }
    }
    pub fn get_month(&self) -> u8 {
        match *self {
            Date(_, m, _, _, _, _, _) => m,
            _ => fail!("Called `Value::get_month()` on non `Date` value")
        }
    }
    pub fn get_day(&self) -> u8 {
        match *self {
            Date(_, _, d, _, _, _, _) => d,
            _ => fail!("Called `Value::get_day()` on non `Date` value")
        }
    }
    pub fn is_time(&self) -> bool {
        match *self {
            Time(..) => true,
            _ => false
        }
    }
    pub fn is_neg(&self) -> bool {
        match *self {
            Time(false, _, _, _, _, _) => false,
            Time(true, _, _, _, _, _) => true,
            _ => fail!("Called `Value::is_neg()` on non `Time` value")
        }
    }
    pub fn get_days(&self) -> u32 {
        match *self {
            Time(_, d, _, _, _, _) => d,
            _ => fail!("Called `Value::get_days()` on non `Time` value")
        }
    }
    pub fn get_hour(&self) -> u8 {
        match *self {
            Date(_, _, _, h, _, _, _) => h,
            Time(_, _, h, _, _, _) => h,
            _ => fail!("Called `Value::get_hour()` on non `Date` nor `Time` value")
        }
    }
    pub fn get_min(&self) -> u8 {
        match *self {
            Date(_, _, _, _, i, _, _) => i,
            Time(_, _, _, i, _, _) => i,
            _ => fail!("Called `Value::get_min()` on non `Date` nor `Time` value")
        }
    }
    pub fn get_sec(&self) -> u8 {
        match *self {
            Date(_, _, _, _, _, s, _) => s,
            Time(_, _, _, _, s, _) => s,
            _ => fail!("Called `Value::get_sec()` on non `Date` nor `Time` value")
        }
    }
    pub fn get_usec(&self) -> u32 {
        match *self {
            Date(_, _, _, _, _, _, u) => u,
            Time(_, _, _, _, _, u) => u,
            _ => fail!("Called `Value::get_usec()` on non `Date` nor `Time` value")
        }
    }
    fn to_bin(&self) -> IoResult<Vec<u8>> {
        let mut writer = MemWriter::with_capacity(256);
        match *self {
            NULL => (),
            Bytes(ref x) => {
                try!(writer.write_lenenc_bytes(x.as_slice()));
            },
            Int(x) => {
                try!(writer.write_le_i64(x));
            },
            UInt(x) => {
                try!(writer.write_le_u64(x));
            },
            Float(x) => {
                try!(writer.write_le_f64(x));
            },
            Date(0u16, 0u8, 0u8, 0u8, 0u8, 0u8, 0u32) => {
                try!(writer.write_u8(0u8));
            },
            Date(y, m, d, 0u8, 0u8, 0u8, 0u32) => {
                try!(writer.write_u8(4u8));
                try!(writer.write_le_u16(y));
                try!(writer.write_u8(m));
                try!(writer.write_u8(d));
            },
            Date(y, m, d, h, i, s, 0u32) => {
                try!(writer.write_u8(7u8));
                try!(writer.write_le_u16(y));
                try!(writer.write_u8(m));
                try!(writer.write_u8(d));
                try!(writer.write_u8(h));
                try!(writer.write_u8(i));
                try!(writer.write_u8(s));
            },
            Date(y, m, d, h, i, s, u) => {
                try!(writer.write_u8(11u8));
                try!(writer.write_le_u16(y));
                try!(writer.write_u8(m));
                try!(writer.write_u8(d));
                try!(writer.write_u8(h));
                try!(writer.write_u8(i));
                try!(writer.write_u8(s));
                try!(writer.write_le_u32(u));
            },
            Time(_, 0u32, 0u8, 0u8, 0u8, 0u32) => try!(writer.write_u8(0u8)),
            Time(neg, d, h, m, s, 0u32) => {
                try!(writer.write_u8(8u8));
                try!(writer.write_u8(if neg {1u8} else {0u8}));
                try!(writer.write_le_u32(d));
                try!(writer.write_u8(h));
                try!(writer.write_u8(m));
                try!(writer.write_u8(s));
            },
            Time(neg, d, h, m, s, u) => {
                try!(writer.write_u8(12u8));
                try!(writer.write_u8(if neg {1u8} else {0u8}));
                try!(writer.write_le_u32(d));
                try!(writer.write_u8(h));
                try!(writer.write_u8(m));
                try!(writer.write_u8(s));
                try!(writer.write_le_u32(u));
            }
        };
        Ok(writer.unwrap())
    }
    #[inline]
    fn from_payload(pld: &[u8], columns_count: uint) -> IoResult<Vec<Value>> {
        let mut output: Vec<Value> = Vec::with_capacity(columns_count);
        let mut reader = BufReader::new(pld);
        loop {
            if reader.eof() {
                break;
            } else if { let pos = try!(reader.tell()); pld[pos as uint] == 0xfb_u8 } {
                try!(reader.seek(1, SeekCur));
                output.push(NULL);
            } else {
                output.push(Bytes(try!(reader.read_lenenc_bytes())));
            }
        }
        Ok(output)
    }
    #[inline]
    fn from_bin_payload(pld: &[u8], columns: &[Column]) -> IoResult<Vec<Value>> {
        let bit_offset = 2; // http://dev.mysql.com/doc/internals/en/null-bitmap.html
        let bitmap_len = (columns.len() + 7 + bit_offset) / 8;
        let mut bitmap: Vec<u8> = Vec::with_capacity(bitmap_len);
        let mut values: Vec<Value> = Vec::with_capacity(columns.len());
        let mut i = -1;
        while {i += 1; i < bitmap_len} {
            bitmap.push(pld[i+1]);
        }
        let mut reader = BufReader::new(pld.slice_from(1 + bitmap_len));
        let mut i = -1;
        while {i += 1; i < columns.len()} {
            if *bitmap.get((i + bit_offset) / 8) & (1 << ((i + bit_offset) % 8)) == 0 {
                values.push(try!(reader.read_bin_value(columns[i].column_type,
                                                                  (columns[i].flags & consts::UNSIGNED_FLAG) != 0)));
            } else {
                values.push(NULL);
            }
        }
        Ok(values)
    }
    // (NULL-bitmap, values, ids of fields to send throwgh send_long_data)
    fn to_bin_payload(params: &[Column], values: &[Value], max_allowed_packet: uint) -> IoResult<(Vec<u8>, Vec<u8>, Option<Vec<u16>>)> {
        let bitmap_len = (params.len() + 7) / 8;
        let mut large_ids: Vec<u16> = Vec::new();
        let mut writer = MemWriter::new();
        let mut bitmap = Vec::from_elem(bitmap_len, 0u8);
        let mut i = 0u16;
        let mut written = 0;
        let cap = max_allowed_packet - bitmap_len - values.len() * 8;
        for value in values.iter() {
            match *value {
                NULL => *bitmap.get_mut(i as uint / 8) |= 1 << (i % 8u16),
                _ => {
                    let val = try!(value.to_bin());
                    if val.len() < cap - written {
                        written += val.len();
                        try!(writer.write(val.as_slice()));
                    } else {
                        large_ids.push(i);
                    }
                }
            }
            i += 1;
        }
        if large_ids.len() == 0 {
            Ok((bitmap, writer.unwrap(), None))
        } else {
            Ok((bitmap, writer.unwrap(), Some(large_ids)))
        }
    }
}

/***
 *    888b     d888           .d88888b.           888             
 *    8888b   d8888          d88P" "Y88b          888             
 *    88888b.d88888          888     888          888             
 *    888Y88888P888 888  888 888     888 88888b.  888888 .d8888b  
 *    888 Y888P 888 888  888 888     888 888 "88b 888    88K      
 *    888  Y8P  888 888  888 888     888 888  888 888    "Y8888b. 
 *    888   "   888 Y88b 888 Y88b. .d88P 888 d88P Y88b.       X88 
 *    888       888  "Y88888  "Y88888P"  88888P"   "Y888  88888P' 
 *                       888             888                      
 *                  Y8b d88P             888                      
 *                   "Y88P"              888                      
 */
#[deriving(Clone, Eq)]
pub struct MyOpts {
    pub tcp_addr: Option<SocketAddr>,
    pub unix_addr: Option<Path>,
    pub user: Option<~str>,
    pub pass: Option<~str>,
    pub db_name: Option<~str>,
    pub prefer_socket: bool,
}

impl MyOpts {
    fn get_user(&self) -> ~str {
        match self.user {
            Some(ref x) => x.clone(),
            None => ~""
        }
    }
    fn get_pass(&self) -> ~str {
        match self.pass {
            Some(ref x) => x.clone(),
            None => ~""
        }
    }
    fn get_db_name(&self) -> ~str {
        match self.db_name {
            Some(ref x) => x.clone(),
            None => ~""
        }
    }
}

impl default::Default for MyOpts {
    fn default() -> MyOpts {
        MyOpts{tcp_addr: Some(SocketAddr{ip: Ipv4Addr(127, 0, 0, 1), port: 3306}),
               unix_addr: None,
               user: None,
               pass: None,
               db_name: None,
               prefer_socket: true}
    }
}

/***
 *    888b     d888           .d8888b.                             
 *    8888b   d8888          d88P  Y88b                            
 *    88888b.d88888          888    888                            
 *    888Y88888P888 888  888 888         .d88b.  88888b.  88888b.  
 *    888 Y888P 888 888  888 888        d88""88b 888 "88b 888 "88b 
 *    888  Y8P  888 888  888 888    888 888  888 888  888 888  888 
 *    888   "   888 Y88b 888 Y88b  d88P Y88..88P 888  888 888  888 
 *    888       888  "Y88888  "Y8888P"   "Y88P"  888  888 888  888 
 *                       888                                       
 *                  Y8b d88P                                       
 *                   "Y88P"                                        
 */

pub struct MyConn {
    opts: MyOpts,
    stream: ~Stream,
    affected_rows: u64,
    last_insert_id: u64,
    max_allowed_packet: uint,
    capability_flags: u32,
    connection_id: u32,
    status_flags: u16,
    seq_id: u8,
    character_set: u8,
    last_command: u8,
    connected: bool
}

impl MyConn {
    pub fn new(opts: MyOpts) -> MyResult<MyConn> {
        if opts.unix_addr.is_some() {
            let unix_stream = UnixStream::connect(opts.unix_addr.get_ref());
            if unix_stream.is_ok() {
                let mut conn = MyConn{
                    stream: ~(unix_stream.unwrap()) as ~Stream,
                    seq_id: 0u8,
                    capability_flags: 0,
                    status_flags: 0u16,
                    connection_id: 0u32,
                    character_set: 0u8,
                    affected_rows: 0u64,
                    last_insert_id: 0u64,
                    last_command: 0u8,
                    max_allowed_packet: consts::MAX_PAYLOAD_LEN,
                    opts: opts,
                    connected: false
                };
                return conn.connect().and(Ok(conn));
            } else {
                return Err(MyStrError(format!("Could not connect to address: {:?}", opts.unix_addr)));
            }
        }
        if opts.tcp_addr.is_some() {
            let tcp_stream = TcpStream::connect(opts.tcp_addr.unwrap());
            if tcp_stream.is_ok() {
                let mut conn = MyConn{
                    stream: ~(tcp_stream.unwrap()) as ~Stream,
                    seq_id: 0u8,
                    capability_flags: 0,
                    status_flags: 0u16,
                    connection_id: 0u32,
                    character_set: 0u8,
                    affected_rows: 0u64,
                    last_insert_id: 0u64,
                    last_command: 0u8,
                    max_allowed_packet: consts::MAX_PAYLOAD_LEN,
                    opts: opts,
                    connected: false
                };
                let res = conn.connect();
                match res {
                    Err(err) => return Err(err),
                    _ => {
                        if conn.opts.prefer_socket &&
                           (conn.opts.tcp_addr.get_ref().ip == Ipv4Addr(127, 0, 0, 1) ||
                            conn.opts.tcp_addr.get_ref().ip == Ipv6Addr(0, 0, 0, 0, 0, 0, 0, 1))
                        {
                            let path = conn.get_system_var("socket");
                            if !path.is_some() {
                                return Ok(conn);
                            } else {
                                let path = path.unwrap().unwrap_bytes();
                                let opts = MyOpts{
                                    unix_addr: Some(Path::new(path)),
                                    ..conn.opts.clone()
                                };
                                return MyConn::new(opts).or(Ok(conn));
                            }
                        } else {
                            return Ok(conn);
                        }
                    }
                }
            } else {
                return Err(MyStrError(format!("Could not connect to address: {:?}", opts.tcp_addr)));
            }
        } else {
            return Err(MyStrError(~"Could not connect. Address not specified"));
        }
    }
    fn handle_handshake(&mut self, hp: &HandshakePacket) {
        self.capability_flags = hp.capability_flags;
        self.status_flags = hp.status_flags;
        self.connection_id = hp.connection_id;
        self.character_set = hp.character_set;
    }
    fn handle_ok(&mut self, op: &OkPacket) {
        self.affected_rows = op.affected_rows;
        self.last_insert_id = op.last_insert_id;
        self.status_flags = op.status_flags;
    }
    fn handle_eof(&mut self, eof: &EOFPacket) {
        self.status_flags = eof.status_flags;
    }
}

impl Reader for MyConn {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<uint> {
        self.stream.read(buf)
    }
}
impl Reader for ~MyConn {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<uint> {
        self.read(buf)
    }
}
impl<'a> Reader for &'a MyConn {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<uint> {
        self.read(buf)
    }
}

impl Writer for MyConn {
    fn write(&mut self, buf: &[u8]) -> IoResult<()> {
        self.stream.write(buf)
    }
}
impl Writer for ~MyConn {
    fn write(&mut self, buf: &[u8]) -> IoResult<()> {
        self.write(buf)
    }
}
impl<'a> Writer for &'a MyConn {
    fn write(&mut self, buf: &[u8]) -> IoResult<()> {
        self.write(buf)
    }
}

/***
 *    888b     d888           .d8888b.  888                                            
 *    8888b   d8888          d88P  Y88b 888                                            
 *    88888b.d88888          Y88b.      888                                            
 *    888Y88888P888 888  888  "Y888b.   888888 888d888  .d88b.   8888b.  88888b.d88b.  
 *    888 Y888P 888 888  888     "Y88b. 888    888P"   d8P  Y8b     "88b 888 "888 "88b 
 *    888  Y8P  888 888  888       "888 888    888     88888888 .d888888 888  888  888 
 *    888   "   888 Y88b 888 Y88b  d88P Y88b.  888     Y8b.     888  888 888  888  888 
 *    888       888  "Y88888  "Y8888P"   "Y888 888      "Y8888  "Y888888 888  888  888 
 *                       888                                                           
 *                  Y8b d88P                                                           
 *                   "Y88P"                                                            
 */

pub trait MyStream: MyReader + MyWriter {
    fn read_packet(&mut self) -> MyResult<Vec<u8>>;
    fn write_packet(&mut self, data: &Vec<u8>) -> MyResult<()>;
    fn handle_ok(&mut self, ok: &OkPacket);
    fn handle_eof(&mut self, eof: &EOFPacket);
    fn handle_handshake(&mut self, hp: &HandshakePacket);
    fn do_handshake(&mut self) -> MyResult<()>;
    fn do_handshake_response(&mut self, hp: &HandshakePacket) -> MyResult<()>;
    fn write_command(&mut self, cmd: u8) -> MyResult<()>;
    fn write_command_data(&mut self, cmd: u8, buf: &[u8]) -> MyResult<()>;
    fn send_local_infile(&mut self, file_name: &[u8]) -> MyResult<()>;
    fn query<'a>(&'a mut self, query: &str) -> MyResult<Option<QueryResult<'a>>>;
    fn prepare(&mut self, query: &str) -> MyResult<Stmt>;
    fn send_long_data(&mut self, stmt: &Stmt, params: &[Value], ids: Vec<u16>) -> MyResult<()>;
    fn execute<'a>(&'a mut self, stmt: &Stmt, params: &[Value]) -> MyResult<Option<QueryResult<'a>>>;
    fn connect(&mut self) -> MyResult<()>;
    fn get_system_var(&mut self, name: &str) -> Option<Value>;
}

impl MyStream for MyConn {
    fn read_packet(&mut self) -> MyResult<Vec<u8>> {
        let mut output = Vec::new();
        loop {
            let payload_len = try_io!(self.read_le_uint_n(3));
            let seq_id = try_io!(self.read_u8());
            if seq_id != self.seq_id {
                return Err(MyStrError(~"Packet out of sync"));
            }
            self.seq_id += 1;
            if payload_len as uint >= consts::MAX_PAYLOAD_LEN {
                try_io!(self.push_exact(&mut output, consts::MAX_PAYLOAD_LEN));
            } else if payload_len == 0 {
                break;
            } else {
                try_io!(self.push_exact(&mut output, payload_len as uint));
                break;
            }
        }
        Ok(output)
    }
    fn write_packet(&mut self, data: &Vec<u8>) -> MyResult<()> {
        if data.len() > self.max_allowed_packet && self.max_allowed_packet < consts::MAX_PAYLOAD_LEN {
            return Err(MyStrError(~"Packet too large"));
        }
        if data.len() == 0 {
            try_io!(self.write([0u8, 0u8, 0u8, self.seq_id]));
            self.seq_id += 1;
            return Ok(());
        }
        let mut last_was_max = false;
        for chunk in data.as_slice().chunks(consts::MAX_PAYLOAD_LEN) {
            let chunk_len = chunk.len();
            let full_chunk_len = 4 + chunk_len;
            let mut full_chunk: Vec<u8> = Vec::from_elem(full_chunk_len, 0u8);
            if chunk_len == consts::MAX_PAYLOAD_LEN {
                last_was_max = true;
                *full_chunk.get_mut(0) = 255u8;
                *full_chunk.get_mut(1) = 255u8;
                *full_chunk.get_mut(2) = 255u8;
            } else {
                last_was_max = false;
                *full_chunk.get_mut(0) = (chunk_len & 255) as u8;
                *full_chunk.get_mut(1) = ((chunk_len & (255 << 8)) >> 8) as u8;
                *full_chunk.get_mut(2) = ((chunk_len & (255 << 16)) >> 16) as u8;
            }
            *full_chunk.get_mut(3) = self.seq_id;
            self.seq_id += 1;
            unsafe {
                let payload_slice = full_chunk.mut_slice_from(4);
                payload_slice.copy_memory(chunk);
            }
            try_io!(self.write(full_chunk.as_slice()));
        }
        if last_was_max {
            try_io!(self.write([0u8, 0u8, 0u8, self.seq_id]));
            self.seq_id += 1;
        }
        Ok(())
    }
    fn handle_handshake(&mut self, hp: &HandshakePacket) {
        self.capability_flags = hp.capability_flags;
        self.status_flags = hp.status_flags;
        self.connection_id = hp.connection_id;
        self.character_set = hp.character_set;
    }
    fn handle_ok(&mut self, op: &OkPacket) {
        self.affected_rows = op.affected_rows;
        self.last_insert_id = op.last_insert_id;
        self.status_flags = op.status_flags;
    }
    fn handle_eof(&mut self, eof: &EOFPacket) {
        self.status_flags = eof.status_flags;
    }
    fn do_handshake(&mut self) -> MyResult<()> {
        self.read_packet().and_then(|pld| {
            let handshake = try_io!(HandshakePacket::from_payload(pld.as_slice()));
            if handshake.protocol_version != 10u8 {
                return Err(MyStrError(format!("Unsupported protocol version {:u}", handshake.protocol_version)));
            }
            if (handshake.capability_flags & consts::CLIENT_PROTOCOL_41) == 0 {
                return Err(MyStrError(~"Server must set CLIENT_PROTOCOL_41 flag"));
            }
            self.handle_handshake(&handshake);
            self.do_handshake_response(&handshake)
        }).and_then(|_| {
            self.read_packet()
        }).and_then(|pld| {
            match *pld.get(0) {
                0u8 => {
                    let ok = try_io!(OkPacket::from_payload(pld.as_slice()));
                    self.handle_ok(&ok);
                    return Ok(());
                },
                0xffu8 => {
                    let err = try_io!(ErrPacket::from_payload(pld.as_slice()));
                    return Err(MySqlError(err));
                },
                _ => return Err(MyStrError(~"Unexpected packet"))
            }
        })
    }
    fn do_handshake_response(&mut self, hp: &HandshakePacket) -> MyResult<()> {
        let mut client_flags = consts::CLIENT_PROTOCOL_41 |
                           consts::CLIENT_SECURE_CONNECTION |
                           consts::CLIENT_LONG_PASSWORD |
                           consts::CLIENT_TRANSACTIONS |
                           consts::CLIENT_LOCAL_FILES |
                           (self.capability_flags & consts::CLIENT_LONG_FLAG);
        let scramble_buf = scramble(hp.auth_plugin_data.as_slice(), self.opts.get_pass().into_bytes()).unwrap();
        let scramble_buf_len = 20;
        let mut payload_len = 4 + 4 + 1 + 23 + self.opts.get_user().len() + 1 + 1 + scramble_buf_len;
        if self.opts.get_db_name().len() > 0 {
            client_flags |= consts::CLIENT_CONNECT_WITH_DB;
            payload_len += self.opts.get_db_name().len() + 1;
        }

        let mut writer = MemWriter::with_capacity(payload_len);
        try_io!(writer.write_le_u32(client_flags));
        try_io!(writer.write_le_u32(0u32));
        try_io!(writer.write_u8(consts::UTF8_GENERAL_CI));
        try_io!(writer.write(~[0u8, ..23]));
        try_io!(writer.write_str(self.opts.get_user()));
        try_io!(writer.write_u8(0u8));
        try_io!(writer.write_u8(scramble_buf_len as u8));
        try_io!(writer.write(scramble_buf));
        if self.opts.get_db_name().len() > 0 {
            try_io!(writer.write_str(self.opts.get_db_name()));
            try_io!(writer.write_u8(0u8));
        }

        self.write_packet(&writer.unwrap())
    }
    fn write_command(&mut self, cmd: u8) -> MyResult<()> {
        self.seq_id = 0u8;
        self.last_command = cmd;
        self.write_packet(&vec!(cmd))
    }
    fn write_command_data(&mut self, cmd: u8, buf: &[u8]) -> MyResult<()> {
        self.seq_id = 0u8;
        self.last_command = cmd;
        self.write_packet(&vec!(cmd).append(buf))
    }
    fn send_long_data(&mut self, stmt: &Stmt, params: &[Value], ids: Vec<u16>) -> MyResult<()> {
        for &id in ids.iter() {
            match params[id as uint] {
                Bytes(ref x) => {
                    for chunk in x.as_slice().chunks(self.max_allowed_packet - 7) {
                        let chunk_len = chunk.len() + 7;
                        let mut writer = MemWriter::with_capacity(chunk_len);
                        try_io!(writer.write_le_u32(stmt.statement_id));
                        try_io!(writer.write_le_u16(id));
                        try_io!(writer.write(chunk));
                        try!(self.write_command_data(consts::COM_STMT_SEND_LONG_DATA, writer.unwrap().as_slice()));
                    }
                },
                _ => (/* quite strange so do nothing */)
            }
        }
        Ok(())
    }
    fn execute<'a>(&'a mut self, stmt: &Stmt, params: &[Value]) -> MyResult<Option<QueryResult<'a>>> {
        if stmt.num_params != params.len() as u16 {
            return Err(MyStrError(format!("Statement takes {:u} parameters but {:u} was supplied", stmt.num_params, params.len())));
        }
        let mut writer = MemWriter::new();
        try_io!(writer.write_le_u32(stmt.statement_id));
        try_io!(writer.write_u8(0u8));
        try_io!(writer.write_le_u32(1u32));
        if stmt.num_params > 0 {
            let (bitmap, values, large_ids) = try_io!(Value::to_bin_payload(stmt.params.get_ref().as_slice(),
                                                                    params,
                                                                    self.max_allowed_packet));
            if large_ids.is_some() {
                try!(self.send_long_data(stmt, params, large_ids.unwrap()));
            }
            try_io!(writer.write(bitmap.as_slice()));
            try_io!(writer.write_u8(1u8));
            let mut i = -1;
            while { i += 1; i < params.len() } {
                let _ = match params[i] {
                    NULL => try_io!(writer.write([stmt.params.get_ref().get(i).column_type, 0u8])),
                    Bytes(..) => try_io!(writer.write([consts::MYSQL_TYPE_VAR_STRING, 0u8])),
                    Int(..) => try_io!(writer.write([consts::MYSQL_TYPE_LONGLONG, 0u8])),
                    UInt(..) => try_io!(writer.write([consts::MYSQL_TYPE_LONGLONG, 128u8])),
                    Float(..) => try_io!(writer.write([consts::MYSQL_TYPE_DOUBLE, 0u8])),
                    Date(..) => try_io!(writer.write([consts::MYSQL_TYPE_DATE, 0u8])),
                    Time(..) => try_io!(writer.write([consts::MYSQL_TYPE_TIME, 0u8]))
                };
            }
            try_io!(writer.write(values.as_slice()));
        }
        try!(self.write_command_data(consts::COM_STMT_EXECUTE, writer.unwrap().as_slice()));
        let pld = try!(self.read_packet());
        match *pld.get(0) {
            0u8 => {
                let ok = try_io!(OkPacket::from_payload(pld.as_slice()));
                self.handle_ok(&ok);
                Ok(None)
            },
            0xffu8 => {
                let err = try_io!(ErrPacket::from_payload(pld.as_slice()));
                Err(MySqlError(err))
            },
            _ => {
                let mut reader = BufReader::new(pld.as_slice());
                let column_count = try_io!(reader.read_lenenc_int());
                let mut columns: Vec<Column> = Vec::with_capacity(column_count as uint);
                let mut i = -1;
                while { i += 1; i < column_count } {
                    let pld = try!(self.read_packet());
                    //let pld = match self.read_packet() {
                    //    Ok(pld) => pld,
                    //    Err(error) => return Err(error)
                    //};
                    columns.push(try_io!(Column::from_payload(self.last_command, pld.as_slice())));
                }
                try!(self.read_packet());
                return Ok(Some(QueryResult{conn: self,
                                 columns: columns,
                                 eof: false,
                                 is_bin: true}));
            }
        }
    }
    fn send_local_infile(&mut self, file_name: &[u8]) -> MyResult<()> {
        let path = Path::new(file_name);
        let mut file = try_io!(File::open(&path));
        let mut chunk = Vec::from_elem(self.max_allowed_packet, 0u8);
        let mut r = file.read(chunk.as_mut_slice());
        loop {
            match r {
                Ok(cnt) => {
                    try!(self.write_packet(&Vec::from_slice(chunk.slice_to(cnt))));
                },
                Err(e) => {
                    if e.kind == EndOfFile {
                        break;
                    } else {
                        return Err(MyIoError(e));
                    }
                }
            }
            r = file.read(chunk.as_mut_slice());
        }
        try!(self.write_packet(&Vec::with_capacity(0)));
        let pld = try!(self.read_packet());
        if *pld.get(0) == 0u8 {
            self.handle_ok(&try_io!(OkPacket::from_payload(pld.as_slice())));
        }
        Ok(())
    }
    fn query<'a>(&'a mut self, query: &str) -> MyResult<Option<QueryResult<'a>>> {
        try!(self.write_command_data(consts::COM_QUERY, query.as_bytes()));
        let pld = try!(self.read_packet());
        match *pld.get(0) {
            0u8 => {
                let ok = try_io!(OkPacket::from_payload(pld.as_slice()));
                self.handle_ok(&ok);
                return Ok(None);
            },
            0xfb_u8 => {
                let mut reader = BufReader::new(pld.as_slice());
                try_io!(reader.seek(1, SeekCur));
                let file_name = try_io!(reader.read_to_end());
                return match self.send_local_infile(file_name.as_slice()) {
                    Ok(..) => Ok(None),
                    Err(err) => Err(err)
                };
            },
            0xff_u8 => {
                let err = try_io!(ErrPacket::from_payload(pld.as_slice()));
                return Err(MySqlError(err));
            },
            _ => {
                let mut reader = BufReader::new(pld.as_slice());
                let column_count = try_io!(reader.read_lenenc_int());
                let mut columns: Vec<Column> = Vec::with_capacity(column_count as uint);
                let mut i = -1;
                while { i += 1; i < column_count } {
                    let pld = try!(self.read_packet());
                    columns.push(try_io!(Column::from_payload(self.last_command, pld.as_slice())));
                }
                // skip eof packet
                try!(self.read_packet());
                return Ok(Some(QueryResult{conn: self,
                                        columns: columns,
                                        eof: false,
                                        is_bin: false}));
            }
        }
    }
    fn prepare(&mut self, query: &str) -> MyResult<Stmt> {
        try!(self.write_command_data(consts::COM_STMT_PREPARE, query.as_bytes()));
        let pld = try!(self.read_packet());
        match *pld.get(0) {
            0xff => {
                let err = try_io!(ErrPacket::from_payload(pld.as_slice()));
                return Err(MySqlError(err));
            },
            _ => {
                let mut stmt = try_io!(Stmt::from_payload(pld.as_slice()));
                if stmt.num_params > 0 {
                    let mut params: Vec<Column> = Vec::with_capacity(stmt.num_params as uint);
                    let mut i = -1;
                    while { i += 1; i < stmt.num_params } {
                        let pld = try!(self.read_packet());
                        params.push(try_io!(Column::from_payload(self.last_command, pld.as_slice())));
                    }
                    stmt.params = Some(params);
                    try!(self.read_packet());
                }
                if stmt.num_columns > 0 {
                    let mut columns: Vec<Column> = Vec::with_capacity(stmt.num_columns as uint);
                    let mut i = -1;
                    while { i += 1; i < stmt.num_columns } {
                        let pld = try!(self.read_packet());
                        columns.push(try_io!(Column::from_payload(self.last_command, pld.as_slice())));
                    }
                    stmt.columns = Some(columns);
                    try!(self.read_packet());
                }
                return Ok(stmt);
            }
        }
    }
    fn connect(&mut self) -> MyResult<()> {
        if self.connected {
            return Ok(());
        }
        self.do_handshake().and_then(|_| {
            let max_allowed_packet = self.get_system_var("max_allowed_packet")
                                         .unwrap_or(NULL)
                                         .unwrap_bytes_or(Vec::with_capacity(0));
            Ok(uint::parse_bytes(max_allowed_packet.as_slice(), 10).unwrap_or(0))
        }).and_then(|max_allowed_packet| {
            if max_allowed_packet == 0 {
                Err(MyStrError(~"Can't get max_allowed_packet value"))
            } else {
                self.max_allowed_packet = max_allowed_packet;
                self.connected = true;
                Ok(())
            }
        })
    }
    fn get_system_var(&mut self, name: &str) -> Option<Value> {
        for row in &mut self.query(format!("SELECT @@{:s};", name)) {
            if row.is_ok() {
                let mut row = row.unwrap();
                return row.shift();
            } else {
                return None;
            }
        }
        return None;
    }
}

/***
 *    888b     d888          8888888b.                             888 888    
 *    8888b   d8888          888   Y88b                            888 888    
 *    88888b.d88888          888    888                            888 888    
 *    888Y88888P888 888  888 888   d88P  .d88b.  .d8888b  888  888 888 888888 
 *    888 Y888P 888 888  888 8888888P"  d8P  Y8b 88K      888  888 888 888    
 *    888  Y8P  888 888  888 888 T88b   88888888 "Y8888b. 888  888 888 888    
 *    888   "   888 Y88b 888 888  T88b  Y8b.          X88 Y88b 888 888 Y88b.  
 *    888       888  "Y88888 888   T88b  "Y8888   88888P'  "Y88888 888  "Y888 
 *                       888                                                  
 *                  Y8b d88P                                                  
 *                   "Y88P"                                                   
 */

pub struct QueryResult<'a> {
    conn: &'a mut MyConn,
    columns: Vec<Column>,
    eof: bool,
    is_bin: bool
}

impl<'a> QueryResult<'a> {
    pub fn next(&mut self) -> Option<MyResult<Vec<Value>>> {
        if self.eof {
            return None
        }
        let pld = match self.conn.read_packet() {
                Err(err) => {
                    self.eof = true;
                    return Some(Err(err));
                },
                Ok(pld) => pld
        };
        if self.is_bin {
            if *pld.get(0) == 0xfe && pld.len() < 0xfe {
                self.eof = true;
                let p = EOFPacket::from_payload(pld.as_slice());
                match p {
                    Ok(p) => {
                        self.conn.handle_eof(&p);
                    },
                    Err(e) => return Some(Err(MyIoError(e)))
                }
                return None;
            }
            let res = Value::from_bin_payload(pld.as_slice(), self.columns.as_slice());
            match res {
                Ok(p) => Some(Ok(p)),
                Err(e) => {
                    self.eof = true;
                    return Some(Err(MyIoError(e)));
                }
            }
        } else {
            if (*pld.get(0) == 0xfe_u8 || *pld.get(0) == 0xff_u8) && pld.len() < 0xfe {
                self.eof = true;
                if *pld.get(0) == 0xfe_u8 {
                    let p = EOFPacket::from_payload(pld.as_slice());
                    match p {
                        Ok(p) => {
                            self.conn.handle_eof(&p);
                        },
                        Err(e) => return Some(Err(MyIoError(e)))
                    }
                    return None;
                } else if *pld.get(0) == 0xff_u8 {
                    let p = ErrPacket::from_payload(pld.as_slice());
                    match p {
                        Ok(p) => return Some(Err(MySqlError(p))),
                        Err(e) => return Some(Err(MyIoError(e)))
                    }
                }
            }
            let res = Value::from_payload(pld.as_slice(), self.columns.len());
            match res {
                Ok(p) => Some(Ok(p)),
                Err(e) => {
                    self.eof = true;
                    Some(Err(MyIoError(e)))
                }
            }
        }
    }
}

#[unsafe_destructor]
impl<'a> Drop for QueryResult<'a> {
    fn drop(&mut self) {
        for _ in *self {}
    }
}

impl<'a> Iterator<MyResult<Vec<Value>>> for &'a mut MyResult<Option<QueryResult<'a>>> {
    fn next(&mut self) -> Option<MyResult<Vec<Value>>> {
        if self.is_ok() {
            let result_opt = self.as_mut().unwrap();
            if result_opt.is_some() {
                let result = result_opt.as_mut().unwrap();
                return result.next();
            }
        }
        return None;
    }
}

/***
 *    88888888888                   888             
 *        888                       888             
 *        888                       888             
 *        888      .d88b.  .d8888b  888888 .d8888b  
 *        888     d8P  Y8b 88K      888    88K      
 *        888     88888888 "Y8888b. 888    "Y8888b. 
 *        888     Y8b.          X88 Y88b.       X88 
 *        888      "Y8888   88888P'  "Y888  88888P' 
 *                                                  
 *                                                  
 *                                                  
 */

#[cfg(test)]
mod test {
    use test::{Bencher};
    use std::default::{Default};
    use std::{str};
    use std::os::{getcwd};
    use std::io::fs::{File, unlink};
    use std::path::posix::{Path};
    use conn::{OkPacket, ErrPacket, EOFPacket, HandshakePacket,
               MyConn, MyStream, MyOpts,
               Bytes, Int, UInt, Date, Time, Float, NULL};

    #[test]
    fn test_ok_packet() {
        let payload = ~[0u8, 1u8, 2u8, 3u8, 0u8, 4u8, 0u8, 32u8];
        let ok_packet = OkPacket::from_payload(payload);
        assert!(ok_packet.is_ok());
        let ok_packet = ok_packet.unwrap();
        assert!(ok_packet.affected_rows == 1);
        assert!(ok_packet.last_insert_id == 2);
        assert!(ok_packet.status_flags == 3);
        assert!(ok_packet.warnings == 4);
        assert!(ok_packet.info == vec!(32u8));
    }

    #[test]
    fn test_err_packet() {
        let payload = ~[255u8, 1u8, 0u8, 35u8, 51u8, 68u8, 48u8, 48u8, 48u8, 32u8, 32u8];
        let err_packet = ErrPacket::from_payload(payload);
        assert!(err_packet.is_ok());
        let err_packet = err_packet.unwrap();
        assert!(err_packet.error_code == 1);
        assert!(err_packet.sql_state == vec!(51u8, 68u8, 48u8, 48u8, 48u8));
        assert!(err_packet.error_message == vec!(32u8, 32u8));
    }

    #[test]
    fn test_eof_packet() {
        let payload = ~[0xfe_u8, 1u8, 0u8, 2u8, 0u8];
        let eof_packet = EOFPacket::from_payload(payload);
        assert!(eof_packet.is_ok());
        let eof_packet = eof_packet.unwrap();
        assert!(eof_packet.warnings == 1);
        assert!(eof_packet.status_flags == 2);
    }

    #[test]
    fn test_handshake_packet() {
        let payload = ~[0x0a_u8,
                        32u8, 32u8, 32u8, 32u8, 0u8,
                        1u8, 0u8, 0u8, 0u8,
                        1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8,
                        0u8,
                        3u8, 0x80_u8];
        let handshake_packet = HandshakePacket::from_payload(payload);
        assert!(handshake_packet.is_ok());
        let handshake_packet = handshake_packet.unwrap();
        assert!(handshake_packet.protocol_version == 0x0a);
        assert!(handshake_packet.connection_id == 1);
        assert!(handshake_packet.auth_plugin_data == vec!(1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8));
        assert!(handshake_packet.capability_flags == 0x00008003);
        let mut payload = Vec::from_slice(payload);
        payload.push(33u8);
        payload.push_all_move(vec!(4u8, 0u8));
        payload.push_all_move(vec!(0x08_u8, 0u8));
        payload.push_all_move(vec!(0x15_u8));
        payload.push_all_move(::std::vec::Vec::from_elem(10, 0u8));
        payload.push_all_move(vec!(0x26_u8, 0x3a_u8, 0x34_u8, 0x34_u8, 0x46_u8, 0x44_u8,
                                0x63_u8, 0x44_u8, 0x69_u8, 0x63_u8, 0x39_u8, 0x30_u8, 0x00_u8));
        payload.push_all_move(vec!(1u8, 2u8, 3u8, 4u8, 5u8, 0u8));
        let handshake_packet = HandshakePacket::from_payload(payload.as_slice());
        assert!(handshake_packet.is_ok());
        let handshake_packet = handshake_packet.unwrap();
        assert!(handshake_packet.protocol_version == 0x0a);
        assert!(handshake_packet.connection_id == 1);
        assert!(handshake_packet.auth_plugin_data == vec!(1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8,
                                                       0x26_u8, 0x3a_u8, 0x34_u8, 0x34_u8, 0x46_u8, 0x44_u8,
                                                       0x63_u8, 0x44_u8, 0x69_u8, 0x63_u8, 0x39_u8, 0x30_u8));
        assert!(handshake_packet.capability_flags == 0x00088003);
        assert!(handshake_packet.character_set == 33);
        assert!(handshake_packet.status_flags == 4);
        assert!(handshake_packet.auth_plugin_name == vec!(1u8, 2u8, 3u8, 4u8, 5u8));
    }

    #[test]
    fn test_value_into_str() {
        let v = NULL;
        assert!(v.into_str() == ~"NULL");
        let v = Bytes(Vec::from_slice((~"hello").into_bytes()));
        assert!(v.into_str() == ~"'hello'");
        let v = Bytes(Vec::from_slice((~"h'e'l'l'o").into_bytes()));
        assert!(v.into_str() == ~"'h\'e\'l\'l\'o'");
        let v = Bytes(vec!(0, 1, 2, 3, 4, 255));
        assert!(v.into_str() == ~"0x0001020304FF");
        let v = Int(-65536);
        assert!(v.into_str() == ~"-65536");
        let v = UInt(4294967296);
        assert!(v.into_str() == ~"4294967296");
        let v = Float(686.868);
        assert!(v.into_str() == ~"686.868");
        let v = Date(0, 0, 0, 0, 0, 0, 0);
        assert!(v.into_str() == ~"''");
        let v = Date(2014, 2, 20, 0, 0, 0, 0);
        assert!(v.into_str() == ~"'2014-02-20'");
        let v = Date(2014, 2, 20, 22, 0, 0, 0);
        assert!(v.into_str() == ~"'2014-02-20 22:00:00'");
        let v = Date(2014, 2, 20, 22, 0, 0, 1);
        assert!(v.into_str() == ~"'2014-02-20 22:00:00.000001'");
        let v = Time(false, 0, 0, 0, 0, 0);
        assert!(v.into_str() == ~"''");
        let v = Time(true, 34, 3, 2, 1, 0);
        assert!(v.into_str() == ~"'-34 003:02:01'");
        let v = Time(false, 10, 100, 20, 30, 40);
        assert!(v.into_str() == ~"'10 100:20:30.000040'");
    }

    #[test]
    fn test_connect() {
        let conn = MyConn::new(MyOpts{user: Some(~"root"),
                                          pass: Some(~"password"),
                                          ..Default::default()});
        assert!(conn.is_ok());
    }

    #[test]
    fn test_connect_with_db() {
        let mut conn = MyConn::new(MyOpts{user: Some(~"root"),
                                          pass: Some(~"password"),
                                          db_name: Some(~"mysql"),
                                          ..Default::default()}).unwrap();
        for x in &mut conn.query("SELECT DATABASE()") {
            assert!(x.unwrap().shift().unwrap().unwrap_bytes() == Vec::from_slice((~"mysql").into_bytes()));
        }
    }

    #[test]
    fn test_query() {
        let mut conn = MyConn::new(MyOpts{user: Some(~"root"),
                                          pass: Some(~"password"),
                                          ..Default::default()}).unwrap();
        assert!(conn.query("DROP DATABASE IF EXISTS test").is_ok());
        assert!(conn.query("CREATE DATABASE test").is_ok());
        assert!(conn.query("USE test").is_ok());
        assert!(conn.query("CREATE TABLE tbl(a TEXT, b INT, c INT UNSIGNED, d DATE, e FLOAT)").is_ok());
        assert!(conn.query("INSERT INTO tbl(a, b, c, d, e) VALUES ('hello', -123, 123, '2014-05-05', 123.123)").is_ok());
        assert!(conn.query("INSERT INTO tbl(a, b, c, d, e) VALUES ('world', -321, 321, '2014-06-06', 321.321)").is_ok());
        assert!(conn.query("SELECT * FROM unexisted").is_err());
        assert!(conn.query("SELECT * FROM tbl").is_ok());
        // Drop
        assert!(conn.query("SELECT * FROM tbl").is_ok());
        assert!(conn.query("UPDATE tbl SET a = 'foo';").is_ok());
        assert!(conn.affected_rows == 2);
        for _ in &mut conn.query("SELECT * FROM tbl WHERE a = 'bar'") {
            assert!(false);
        }
        let mut count = 0;
        for row in &mut conn.query("SELECT * FROM tbl") {
            assert!(row.is_ok());
            let row = row.unwrap();
            if count == 0 {
                assert!(*row.get(0) == Bytes(Vec::from_slice((~"foo").into_bytes())));
                assert!(*row.get(1) == Bytes(Vec::from_slice((~"-123").into_bytes())));
                assert!(*row.get(2) == Bytes(Vec::from_slice((~"123").into_bytes())));
                assert!(*row.get(3) == Bytes(Vec::from_slice((~"2014-05-05").into_bytes())));
                assert!(*row.get(4) == Bytes(Vec::from_slice((~"123.123").into_bytes())));
            } else {
                assert!(*row.get(0) == Bytes(Vec::from_slice((~"foo").into_bytes())));
                assert!(*row.get(1) == Bytes(Vec::from_slice((~"-321").into_bytes())));
                assert!(*row.get(2) == Bytes(Vec::from_slice((~"321").into_bytes())));
                assert!(*row.get(3) == Bytes(Vec::from_slice((~"2014-06-06").into_bytes())));
                assert!(*row.get(4) == Bytes(Vec::from_slice((~"321.321").into_bytes())));
            }
            count += 1;
        }
        assert!(count == 2);
        for row in &mut conn.query("SELECT REPEAT('A', 20000000)") {
            assert!(row.is_ok());
            let row = row.unwrap();
            let val = row.get(0).bytes_ref();
            assert!(val.len() == 20000000);
            for y in val.iter() {
                assert!(y == &65u8);
            }
        }
    }

    #[test]
    fn test_prepared_statemenst() {
        let mut conn = MyConn::new(MyOpts{user: Some(~"root"),
                                          pass: Some(~"password"),
                                          ..Default::default()}).unwrap();
        assert!(conn.query("DROP DATABASE IF EXISTS test").is_ok());
        assert!(conn.query("CREATE DATABASE test").is_ok());
        assert!(conn.query("USE test").is_ok());
        assert!(conn.query("CREATE TABLE tbl(a TEXT, b INT, c INT UNSIGNED, d DATE, e DOUBLE)").is_ok());
        let stmt = conn.prepare("INSERT INTO tbl(a, b, c, d, e) VALUES (?, ?, ?, ?, ?)");
        assert!(stmt.is_ok());
        let stmt = stmt.unwrap();
        assert!(conn.execute(&stmt, [Bytes(Vec::from_slice((~"hello").into_bytes())), Int(-123), UInt(123), Date(2014, 5, 5,0,0,0,0), Float(123.123f64)]).is_ok());
        assert!(conn.execute(&stmt, [Bytes(Vec::from_slice((~"world").into_bytes())), NULL, NULL, NULL, Float(321.321f64)]).is_ok());
        let stmt = conn.prepare("SELECT * FROM tbl");
        assert!(stmt.is_ok());
        let stmt = stmt.unwrap();
        let mut i = 0;
        for row in &mut conn.execute(&stmt, []) {
            assert!(row.is_ok());
            let row = row.unwrap();
            if i == 0 {
                assert!(*row.get(0) == Bytes(vec!(104u8, 101u8, 108u8, 108u8, 111u8)));
                assert!(*row.get(1) == Int(-123i64));
                assert!(*row.get(2) == Int(123i64));
                assert!(*row.get(3) == Date(2014u16, 5u8, 5u8, 0u8, 0u8, 0u8, 0u32));
                assert!(row.get(4).get_float() == 123.123);
            } else {
                assert!(*row.get(0) == Bytes(vec!(119u8, 111u8, 114u8, 108u8, 100u8)));
                assert!(*row.get(1) == NULL);
                assert!(*row.get(2) == NULL);
                assert!(*row.get(3) == NULL);
                assert!(row.get(4).get_float() == 321.321);
            }
            i += 1;
        }
        let stmt = conn.prepare("SELECT REPEAT('A', 20000000);");
        let stmt = stmt.unwrap();
        for row in &mut conn.execute(&stmt, []) {
            assert!(row.is_ok());
            let row = row.unwrap();
            let v: &[u8] = row.get(0).bytes_ref();
            assert!(v.len() == 20000000);
            for y in v.iter() {
                assert!(y == &65u8);
            }
        }
    }

    #[test]
    fn test_large_insert() {
        let mut conn = MyConn::new(MyOpts{user: Some(~"root"),
                                          pass: Some(~"password"),
                                          ..Default::default()}).unwrap();
        assert!(conn.query("DROP DATABASE IF EXISTS test").is_ok());
        assert!(conn.query("CREATE DATABASE test").is_ok());
        assert!(conn.query("USE test").is_ok());
        assert!(conn.query("CREATE TABLE tbl(a LONGBLOB)").is_ok());
        let query = format!("INSERT INTO tbl(a) VALUES('{:s}')", str::from_chars(Vec::from_elem(20000000, 'A').as_slice()));
        assert!(conn.query(query).is_ok());
        let x = (&mut conn.query("SELECT * FROM tbl")).next().unwrap();
        assert!(x.is_ok());
        let v: Vec<u8> = x.unwrap().shift().unwrap().unwrap_bytes();
        assert!(v.len() == 20000000);
        for y in v.iter() {
            assert!(y == &65u8);
        }

    }

    #[test]
    fn test_large_insert_prepared() {
        let mut conn = MyConn::new(MyOpts{user: Some(~"root"),
                                          pass: Some(~"password"),
                                          ..Default::default()}).unwrap();
        assert!(conn.query("DROP DATABASE IF EXISTS test").is_ok());
        assert!(conn.query("CREATE DATABASE test").is_ok());
        assert!(conn.query("USE test").is_ok());
        assert!(conn.query("CREATE TABLE tbl(a LONGBLOB)").is_ok());
        let stmt = conn.prepare("INSERT INTO tbl(a) values ( ? );");
        assert!(stmt.is_ok());
        let stmt = stmt.unwrap();
        let val = Vec::from_elem(20000000, 65u8);
        assert!(conn.execute(&stmt, [Bytes(val)]).is_ok());
        let row = (&mut conn.query("SELECT * FROM tbl")).next().unwrap();
        assert!(row.is_ok());
        let row = row.unwrap();
        let v = row.get(0).bytes_ref();
        assert!(v.len() == 20000000);
        for y in v.iter() {
            assert!(y == &65u8);
        }
    }

    #[test]
    #[allow(unused_must_use)]
    fn test_local_infile() {
        let mut conn = MyConn::new(MyOpts{user: Some(~"root"),
                                          pass: Some(~"password"),
                                          ..Default::default()}).unwrap();
        assert!(conn.query("DROP DATABASE IF EXISTS test").is_ok());
        assert!(conn.query("CREATE DATABASE test").is_ok());
        assert!(conn.query("USE test").is_ok());
        assert!(conn.query("CREATE TABLE tbl(a TEXT)").is_ok());
        let mut path = getcwd();
        path.push(~"local_infile.txt");
        {
            let mut file = File::create(&path).unwrap();
            file.write_line(&"AAAAAA");
            file.write_line(&"BBBBBB");
            file.write_line(&"CCCCCC");
        }
        let query = format!("LOAD DATA LOCAL INFILE '{:s}' INTO TABLE tbl", str::from_utf8(path.clone().into_vec().as_slice()).unwrap());
        assert!(conn.query(query).is_ok());
        let mut count = 0;
        for row in &mut conn.query("SELECT * FROM tbl") {
            assert!(row.is_ok());
            let row = row.unwrap();
            match count {
                0 => assert!(row == vec!(Bytes(vec!(65u8, 65u8, 65u8, 65u8, 65u8, 65u8)))),
                1 => assert!(row == vec!(Bytes(vec!(66u8, 66u8, 66u8, 66u8, 66u8, 66u8)))),
                2 => assert!(row == vec!(Bytes(vec!(67u8, 67u8, 67u8, 67u8, 67u8, 67u8)))),
                _ => assert!(false)
            }
            count += 1;
        }
        assert!(count == 3);
        unlink(&path);
    }

    #[bench]
    #[allow(unused_must_use)]
    fn bench_simple_exec(bench: &mut Bencher) {
        let mut conn = MyConn::new(MyOpts{unix_addr: Some(Path::new("/run/mysqld/mysqld.sock")),
                                          user: Some(~"root"),
                                          pass: Some(~"password"),
                                          ..Default::default()}).unwrap();
        bench.iter(|| { conn.query("DO 1"); })
    }

    #[bench]
    #[allow(unused_must_use)]
    fn bench_prepared_exec(bench: &mut Bencher) {
        let mut conn = MyConn::new(MyOpts{unix_addr: Some(Path::new("/run/mysqld/mysqld.sock")),
                                          user: Some(~"root"),
                                          pass: Some(~"password"),
                                          ..Default::default()}).unwrap();
        let stmt = conn.prepare("DO 1").unwrap();
        bench.iter(|| { conn.execute(&stmt, []); })
    }

    #[bench]
    #[allow(unused_must_use)]
    fn bench_simple_query_row(bench: &mut Bencher) {
        let mut conn = MyConn::new(MyOpts{unix_addr: Some(Path::new("/run/mysqld/mysqld.sock")),
                                          user: Some(~"root"),
                                          pass: Some(~"password"),
                                          ..Default::default()}).unwrap();
        bench.iter(|| { (&mut conn.query("SELECT 1")).next(); })
    }

    #[bench]
    #[allow(unused_must_use)]
    fn bench_simple_prepared_query_row(bench: &mut Bencher) {
        let mut conn = MyConn::new(MyOpts{unix_addr: Some(Path::new("/run/mysqld/mysqld.sock")),
                                          user: Some(~"root"),
                                          pass: Some(~"password"),
                                          ..Default::default()}).unwrap();
        let stmt = conn.prepare("SELECT 1").unwrap();
        bench.iter(|| { conn.execute(&stmt, []); })
    }

    #[bench]
    #[allow(unused_must_use)]
    fn bench_simple_prepared_query_row_param(bench: &mut Bencher) {
        let mut conn = MyConn::new(MyOpts{unix_addr: Some(Path::new("/run/mysqld/mysqld.sock")),
                                          user: Some(~"root"),
                                          pass: Some(~"password"),
                                          ..Default::default()}).unwrap();
        let stmt = conn.prepare("SELECT ?").unwrap();
        let mut i = 0;
        bench.iter(|| { conn.execute(&stmt, [Int(i)]); i += 1; })
    }

    #[bench]
    #[allow(unused_must_use)]
    fn bench_prepared_query_row_5param(bench: &mut Bencher) {
        let mut conn = MyConn::new(MyOpts{unix_addr: Some(Path::new("/run/mysqld/mysqld.sock")),
                                          user: Some(~"root"),
                                          pass: Some(~"password"),
                                          ..Default::default()}).unwrap();
        let stmt = conn.prepare("SELECT ?, ?, ?, ?, ?").unwrap();
        let params = ~[Int(42), Bytes(vec!(104u8, 101u8, 108u8, 108u8, 111u8, 111u8)), Float(1.618), NULL, Int(1)];
        bench.iter(|| { conn.execute(&stmt, params); })
    }

    #[bench]
    #[allow(unused_must_use)]
    fn bench_select_large_string(bench: &mut Bencher) {
        let mut conn = MyConn::new(MyOpts{unix_addr: Some(Path::new("/run/mysqld/mysqld.sock")),
                                          user: Some(~"root"),
                                          pass: Some(~"password"),
                                          ..Default::default()}).unwrap();
        bench.iter(|| { for _ in &mut conn.query("SELECT REPEAT('A', 10000)") {} })
    }

    #[bench]
    #[allow(unused_must_use)]
    fn bench_select_prepared_large_string(bench: &mut Bencher) {
        let mut conn = MyConn::new(MyOpts{unix_addr: Some(Path::new("/run/mysqld/mysqld.sock")),
                                          user: Some(~"root"),
                                          pass: Some(~"password"),
                                          ..Default::default()}).unwrap();
        let stmt = conn.prepare("SELECT REPEAT('A', 10000)").unwrap();
        bench.iter(|| { conn.execute(&stmt, []); })
    }
}
