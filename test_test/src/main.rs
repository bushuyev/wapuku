use std::borrow::Cow;
use std::io::Cursor;
use cgmath::{Matrix3, Matrix4, Vector3};
// use polars::prelude::*;
// use polars::io::parquet::*;
// use polars::io::parquet::*;
// use polars::lazy::*;
use polars::prelude::*;
use polars::df;
// use polars::prelude::Expr::Columns;
use polars::time::*;
use polars::time::Duration;

#[cfg(test)]
mod tests {
    // use crate::visualization::MeshModel;
    use polars::df;
    use std::ops::Mul;
    use polars::time::Duration;
    use cgmath::{SquareMatrix, Vector3, Vector4};
    use crate::transform_point;
    use polars::prelude::*;

    #[test]
    pub fn test_build_instances(){
        // let model = MeshModel::new(); 
        let width = 1003.;
        let height = 290.;
        let x = width; ///2.;
        let y = height;///2.;

        let clip_x = x / width  *  2. - 1.;
        let clip_y = y / height * -2. + 1.;

        //[[-1.1914613, 0.0, 0.0, 0.0], [0.0, 2.4142134, 0.0, 0.0], [0.0, 0.0, 1.001001, 1.0], [0.0, 0.0, 9.90991, 10.0]]
        // let m_0 = [
        //     [-1.1914613,    0.0,        0.0,        0.0],
        //     [0.0,           2.4142134,  0.0,        0.0],
        //     [0.0,           0.0,        1.001001,   1.0],
        //     [0.0,           0.0,        9.90991,    10.0]
        // ];
        
        let m_0 = [[-0.6980278, 0.0, 0.0, 0.0], [0.0, 2.4142134, 0.0, 0.0], [0.0, 0.0, 1.001001, 1.0], [0.0, 0.0, 9.90991, 10.0]];

        let proj_m = cgmath::Matrix4::new(
            m_0[0][ 0],  m_0[0][ 1], m_0[0][ 2], m_0[0][ 3],
            m_0[1][ 0],  m_0[1][ 1], m_0[1][ 2], m_0[1][ 3],
            m_0[2][ 0],  m_0[2][ 1], m_0[2][ 2], m_0[2][ 3],
            m_0[3][ 0],  m_0[3][ 1], m_0[3][ 2], m_0[3][ 3],
        );



       /* let point_clip = Vector4::new(clip_x, clip_y, 10., 1.);

        let size_in_world = m.invert().map(|mi|mi.mul(point_clip));
        let size_in_world_2 = m.invert().map(|mi|{
            transform_point(mi, point_clip.truncate())
        });
        // let size_in_world = m.mul(size);

        println!("m={:?} point_clip={:?}, size_in_world={:?} size_in_world_2={:?} m.invert()={:?}", m, point_clip, size_in_world, size_in_world_2, m.invert());*/
        let v = Vector4::new(1., 0., 0., 1.);
        let instance = Vector3::new(0., 0., 0.);
        let model_matrix = cgmath::Matrix4::from_translation(instance);
        let world_position = model_matrix.mul(v);
        
        let v_clip = proj_m.mul(world_position);
        let v_ndc = Vector3::new(v_clip.x/v_clip.w, v_clip.y/v_clip.w, v_clip.z/v_clip.w);

        let x = (v_ndc.x + 1.) * (width / 2.);
        let y = (v_ndc.y + 1.) * (height / 2.);

        println!("m={:?} v_clip={:?} v_ndc={:?}  x={}, y={}", proj_m, v_clip, v_ndc, x, y);

    }
    
    #[test]
    fn test_group(){
        let mut df = df!(
            "field_1" => &[10,      20,     30,     40,   41,    50,     60,     70,     80,     90], 
            "field_2" => &[1,       1,      1,      1,    3,     2,      2,      2,      2,      2],
            "field_3" => &["a",     "b",    "c",    "d",  "dd",  "e",   "f",    "g",    "h",    "ii"],
            "field_4" => &[0.1,     0.1,    0.1,    0.1,  0.1,   0.1,   0.1,    0.1,    0.1,    0.1]
        ).unwrap();

        // df.
        let mut df = df.clone()
            .lazy()
            .groupby_dynamic(
                [col("field_1")],
                DynamicGroupOptions {
                    index_column: "field_1".into(),
                    every: Duration::new(10),
                    period: Duration::new(10),
                    offset: Duration::new(0),
                    truncate: true,
                    include_boundaries: true,
                    closed_window: ClosedWindow::Left,
                    start_by: Default::default(),
                }
            )
            .agg([col("field_2").count()])
            .collect()?;
        println!("df={:?}", df);
    }
}

fn transform_point(mx:Matrix4<f32>, v:Vector3<f32>) -> Vector3<f32>{
    let x:[[f32; 4];4] = mx.into();
    let m = x.into_iter().flat_map(|m|m.into_iter()).collect::<Vec<f32>>();
   
    // println!("m={:?}", m);
    
    let d =   v.x * m[0 * 4 + 3] + v.y * m[1 * 4 + 3] + v.z * m[2 * 4 + 3] + m[3 * 4 + 3];
    
    let dst_0 = (v.x * m[0 * 4 + 0] + v.y * m[1 * 4 + 0] + v.z * m[2 * 4 + 0] + m[3 * 4 + 0]) / d;
    let dst_1 = (v.x * m[0 * 4 + 1] + v.y * m[1 * 4 + 1] + v.z * m[2 * 4 + 1] + m[3 * 4 + 1]) / d;
    let dst_2 = (v.x * m[0 * 4 + 2] + v.y * m[1 * 4 + 2] + v.z * m[2 * 4 + 2] + m[3 * 4 + 2]) / d;

    Vector3::new(dst_0, dst_1, dst_2)
}


fn main() {

    // let df = df!(
    //     "field_1" => &[10,      20,     30,     40,   41,    50,     60,     70,     80,     90], 
    //     "field_2" => &[1,       1,      1,      1,    3,     2,      2,      2,      2,      2],
    //     "field_3" => &["a",     "b",    "c",    "d",  "dd",   "e",    "f",    "g",    "h",    "i"]
    // ).unwrap();
    // let parquet_bytes = include_bytes!("../../wapuku-model/data/d2_transactions_pi_message.par");
    // 
    // let mut buff = Cursor::new(parquet_bytes);
    // 
    // let mut reader = ParquetReader::new(buff);
    // 
    // println!("ok: entries in batch - {:?} ", reader.num_rows());
    println!("Okk");
}
