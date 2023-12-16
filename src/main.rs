use bvh_anim;
use std::error::Error;
use std::fs;
use std::io;

fn main() -> Result<(), Box<dyn Error>> {
    let animation_file_path = "./assets/walking-animation-dataset/";

    if let Ok(entries) = fs::read_dir(animation_file_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(file_name) = entry.file_name().to_str() {
                    println!("================================== Animation File: {} ==================================", file_name);
                    let animation_file_name = animation_file_path.to_owned() + &file_name;

                    let bvh_file: fs::File = fs::File::open(animation_file_name)?;
                    let bvh: bvh_anim::Bvh = bvh_anim::from_reader(io::BufReader::new(bvh_file))?;

                    for joint in bvh.joints() {
                        println!("{:#?}", joint);
                    }

                    for frame in bvh.frames() {
                        println!(
                            "{:#?}",
                            frame.get(&bvh.joints().next().unwrap().data().channels()[0])
                        );
                        break;
                    }

                    println!("Frame time: {:?}", bvh.frame_time());

                    // for frame in bvh.frames() {
                    //     println!("{:?}", frame);
                    // }

                    // let mut out_file = File::create("./out.bvh");
                    // bvh.write_to(&mut out_file)?;
                    break;
                }
            }
        }
        Ok(())
    } else {
        Err("Failed to read directory".into())
    }
}
