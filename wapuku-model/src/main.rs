

pub mod model;

fn main() {
// 
//     let f = File::open("../www/data/wapuku.obj").unwrap();
//     // let f = File::open("/home/bu/dvl/rust/learn-wgpu/code/beginner/tutorial9-models/res/cube.obj").unwrap();
// 
//     let mut obj_reader = BufReader::new(f);
// 
//     let (mesh, mat_r) = tobj::load_obj_buf(
//         &mut obj_reader,
//         &tobj::LoadOptions {
//             triangulate: true,
//             single_index: true,
//             ..Default::default()
//         },
//         |p| {
// 
//             println!("p={:?}", p);
//             let mat_text = "";
//             
//             // println!("mat_text={}", mat_text);
// 
//             let f = File::open("../www/data/wapuku.mtl").unwrap();
//             // let f = File::open("/home/bu/dvl/rust/learn-wgpu/code/beginner/tutorial9-models/res/cube.mtl").unwrap();
// 
//             let mut mtl_reader = BufReader::new(f);
//             
//             tobj::load_mtl_buf(&mut mtl_reader)
//         }
//     ).unwrap();
//     
//     println!("model={:?}", mesh);
//     println!("mat={:?}", mat_r.unwrap());
// 
}




#[cfg(test)]
mod tests {
    use log::LevelFilter;
    use wapuku_model::parquet_scan::parquet_scan;
    use simplelog::{ColorChoice, CombinedLogger, Config, TerminalMode, TermLogger};

    pub fn init_log() {
        CombinedLogger::init(
            vec![
                TermLogger::new(LevelFilter::Trace, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
            ]                                                                                                                                   
        ).unwrap_or_else(|_|{});
    }

    #[ctor::ctor]
    fn init() {
        init_log();
    }
    
    #[test]
    fn test_1() {
        parquet_scan();
        
    }

}
