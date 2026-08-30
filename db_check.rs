use rusqlite::Connection;
fn main() {
    let conn = Connection::open("data.db").unwrap();
    println!("=== devices ===");
    let mut s = conn.prepare("SELECT id, name, ip, status FROM devices").unwrap();
    let _ = s.query_map([], |r| {
        Ok(println!("  id={} name={} ip={} status={}", r.get::<_,i64>(0).unwrap(), r.get::<_,String>(1).unwrap(), r.get::<_,String>(2).unwrap(), r.get::<_,String>(3).unwrap()))
    }).unwrap().for_each(|_|{});
    println!("=== interface_samples per device ===");
    let mut s2 = conn.prepare("SELECT device_id, COUNT(*) FROM interface_samples GROUP BY device_id").unwrap();
    let _ = s2.query_map([], |r| {
        Ok(println!("  device_id={} count={}", r.get::<_,i64>(0).unwrap(), r.get::<_,i64>(1).unwrap()))
    }).unwrap().for_each(|_|{});
    println!("=== device_metrics per device ===");
    let mut s3 = conn.prepare("SELECT device_id, COUNT(*) FROM device_metrics GROUP BY device_id").unwrap();
    let _ = s3.query_map([], |r| {
        Ok(println!("  device_id={} count={}", r.get::<_,i64>(0).unwrap(), r.get::<_,i64>(1).unwrap()))
    }).unwrap().for_each(|_|{});
    println!("=== latest 5 interface_samples ===");
    let mut s4 = conn.prepare("SELECT device_id, sampled_at FROM interface_samples ORDER BY sampled_at DESC LIMIT 5").unwrap();
    let _ = s4.query_map([], |r| {
        Ok(println!("  device_id={} at={}", r.get::<_,i64>(0).unwrap(), r.get::<_,String>(1).unwrap()))
    }).unwrap().for_each(|_|{});
}
