

pub fn load(){
    
}

pub fn resource_filename(url:&str)->&str {
    url.rfind("/").map(|i|&url[i+1..]).unwrap_or(url)
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufReader;
    use log::{debug, LevelFilter};
    use simplelog::{ColorChoice, CombinedLogger, Config, TerminalMode, TermLogger};
    use crate::resources::*;

    //TODO move to util
    pub(crate) fn init_log() {
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
    pub fn test_load(){

        let f = File::open("../wapuku-ui/www/data/wapuku.obj").unwrap();
                // let f = File::open("/home/bu/dvl/rust/learn-wgpu/code/beginner/tutorial9-models/res/cube.obj").unwrap();
    
        let mut obj_reader = BufReader::new(f);
    
        let (model, mat_r) = tobj::load_obj_buf(
            &mut obj_reader,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
            |p| {
    
                println!("p={:?}", p);
                let mat_text = "";
                
                // println!("mat_text={}", mat_text);
                //TODO name
                let f = File::open("../wapuku-ui/www/data/wapuku.mtl").unwrap();
                // let f = File::open("/home/bu/dvl/rust/learn-wgpu/code/beginner/tutorial9-models/res/cube.mtl").unwrap();
    
                let mut mtl_reader = BufReader::new(f);
                
                tobj::load_mtl_buf(&mut mtl_reader)
            }
        ).unwrap();

        println!("model={:?}", model.iter().map(|m|m.name.to_owned()).collect::<Vec<String>>());
        println!("mat={:?}", mat_r.unwrap());
        debug!("Ok");
    }
    
    #[test]
    pub fn test_resource_filename_absolute(){
        let url = "../../../wapuku-resources/blender/wapuku_purple_1024.jpg";
        
        assert_eq!("wapuku_purple_1024.jpg", resource_filename(url));
    }


    #[test]
    pub fn test_resource_filename_relative(){
        let url = "wapuku_purple_1024.jpg";

        assert_eq!("wapuku_purple_1024.jpg", resource_filename(url));
    }
}