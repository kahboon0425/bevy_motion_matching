use bevy::prelude::*;

fn quaternion_difference(q1: Quat, q2: Quat) -> Quat {
    // Calculate the difference quaternion
    q1.inverse() * q2
}

fn main() {
    let q1 = Quat::from_euler(EulerRot::XYZ, 0.0, 0.5, 0.0); // Example quaternion 1 (45° Y rotation)
    let q2 = Quat::from_euler(EulerRot::XYZ, 0.0, 1.0, 0.0); // Example quaternion 2 (90° Y rotation)

    let q_diff = quaternion_difference(q1, q2);

    println!("Quaternion 1: {:?}", q1);
    println!("Quaternion 2: {:?}", q2);
    println!("Quaternion Difference: {:?}", q_diff);
    println!("Q2 reconstructed: {:?}", q1 * q_diff);
}
