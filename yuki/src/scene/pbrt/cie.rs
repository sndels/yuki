// From Simple Analytic Approximations to the CIE XYZ Color Matching Functions
// By Wyman, Sloan and Shirley

pub fn x_fit_1931(lambda: f32) -> f32 {
    let t1 = (lambda - 442.0) * if lambda < 442.0 { 0.0624 } else { 0.0374 };
    let t2 = (lambda - 599.8) * if lambda < 599.8 { 0.0264 } else { 0.0323 };
    let t3 = (lambda - 501.1) * if lambda < 501.1 { 0.0490 } else { 0.0382 };
    0.362 * (-0.5 * t1 * t1).exp() + 1.056 * (-0.5 * t2 * t2).exp() - 0.065 * (-0.5 * t3 * t3).exp()
}

pub fn y_fit_1931(lambda: f32) -> f32 {
    let t1 = (lambda - 568.8) * if lambda < 568.8 { 0.0213 } else { 0.0247 };
    let t2 = (lambda - 530.9) * if lambda < 530.9 { 0.0613 } else { 0.0322 };
    0.821 * (-0.5 * t1 * t1).exp() + 0.286 * (-0.5 * t2 * t2).exp()
}

pub fn z_fit_1931(lambda: f32) -> f32 {
    let t1 = (lambda - 437.0) * if lambda < 437.0 { 0.0845 } else { 0.0278 };
    let t2 = (lambda - 459.0) * if lambda < 459.0 { 0.0385 } else { 0.0725 };
    1.217 * (-0.5 * t1 * t1).exp() + 0.681 * (-0.5 * t2 * t2).exp()
}
