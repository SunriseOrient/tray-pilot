//! 程序化生成托盘图标：圆角方形 + 靛蓝渐变 + 白色闪电。
//!
//! 不依赖任何外部图片资源，直接在内存里做 4×4 超采样得到平滑边缘。

/// 图标边长（像素）。
pub const SIZE: u32 = 64;

/// 闪电轮廓（64×64 坐标系）。
const BOLT: [(f32, f32); 7] = [
    (35.5, 7.0),
    (19.5, 33.0),
    (29.0, 33.0),
    (23.5, 57.0),
    (44.5, 28.0),
    (35.0, 28.0),
    (43.0, 7.0),
];

/// 生成 RGBA 像素缓冲（长度 = SIZE * SIZE * 4）。
pub fn tray_icon_rgba() -> Vec<u8> {
    const SS: usize = 4; // 每个像素每轴的采样数
    const SAMPLES: f32 = (SS * SS) as f32;

    let s = SIZE as usize;
    let mut buf = vec![0u8; s * s * 4];

    for y in 0..s {
        for x in 0..s {
            let mut shape_hits = 0.0f32;
            let mut bolt_hits = 0.0f32;
            let mut gradient_sum = 0.0f32;

            for sy in 0..SS {
                for sx in 0..SS {
                    let px = x as f32 + (sx as f32 + 0.5) / SS as f32;
                    let py = y as f32 + (sy as f32 + 0.5) / SS as f32;

                    if sd_round_rect(px, py, s) <= 0.0 {
                        shape_hits += 1.0;
                        gradient_sum += (px + py) / (2.0 * s as f32);
                        if point_in_polygon(px, py, &BOLT) {
                            bolt_hits += 1.0;
                        }
                    }
                }
            }

            let alpha = shape_hits / SAMPLES;
            let bolt = bolt_hits / SAMPLES;
            let t = if shape_hits > 0.0 {
                gradient_sum / shape_hits
            } else {
                0.0
            };

            // 靛蓝 -> 紫罗兰 的线性渐变
            let (r0, g0, b0) = (99.0f32, 102.0f32, 241.0f32);
            let (r1, g1, b1) = (139.0f32, 92.0f32, 246.0f32);
            let mut r = r0 + (r1 - r0) * t;
            let mut g = g0 + (g1 - g0) * t;
            let mut b = b0 + (b1 - b0) * t;

            // 叠加白色闪电
            r += (255.0 - r) * bolt;
            g += (255.0 - g) * bolt;
            b += (255.0 - b) * bolt;

            let i = (y * s + x) * 4;
            buf[i] = r.round().clamp(0.0, 255.0) as u8;
            buf[i + 1] = g.round().clamp(0.0, 255.0) as u8;
            buf[i + 2] = b.round().clamp(0.0, 255.0) as u8;
            buf[i + 3] = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }

    buf
}

/// 圆角矩形的有符号距离场（内部为负）。
fn sd_round_rect(px: f32, py: f32, size: usize) -> f32 {
    let center = size as f32 * 0.5;
    let half = size as f32 * 0.5 - 2.0;
    let radius = 15.0;

    let dx = (px - center).abs() - (half - radius);
    let dy = (py - center).abs() - (half - radius);
    let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();
    outside + dx.max(dy).min(0.0) - radius
}

/// 射线法判断点是否在多边形内部。
fn point_in_polygon(x: f32, y: f32, poly: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}
