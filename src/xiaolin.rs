// integer part of x
fn ipart(x: f32) -> i32 {
    x.floor() as _
}

fn round(x: f32) -> i32 {
    ipart(x + 0.5) as _
}

// fractional part of x
fn fpart(x: f32) -> f32 {
    x - x.floor()
}

fn rfpart(x: f32) -> f32 {
    1. - fpart(x)
}

/// Draw an antialiased line
/// https://en.wikipedia.org/wiki/Xiaolin_Wu%27s_line_algorithm
pub fn draw_line(
    mut x0: f32,
    mut y0: f32,
    mut x1: f32,
    mut y1: f32,
    mut plot: impl FnMut(i32, i32, f32),
) {
    let steep = (y1 - y0).abs() > (x1 - x0).abs();

    if steep {
        std::mem::swap(&mut x0, &mut y0);
        std::mem::swap(&mut x1, &mut y1);
    }

    if x0 > x1 {
        std::mem::swap(&mut x0, &mut x1);
        std::mem::swap(&mut y0, &mut y1);
    }

    let dx = x1 - x0;
    let dy = y1 - y0;

    let gradient = if dx == 0.0 { 1.0 } else { dy / dx };

    // handle first endpoint
    let mut xend = round(x0);
    let mut yend = y0 + gradient * (xend as f32 - x0);
    let mut xgap = rfpart(x0 + 0.5);
    let xpxl1 = xend; // this will be used in the main loop
    let ypxl1 = ipart(yend);

    if steep {
        plot(ypxl1 as _, xpxl1, rfpart(yend) * xgap);
        plot(ypxl1 + 1, xpxl1, fpart(yend) * xgap);
    } else {
        plot(xpxl1, ypxl1, rfpart(yend) * xgap);
        plot(xpxl1, ypxl1 + 1, fpart(yend) * xgap);
    }

    let mut intery = yend + gradient; // first y-intersection for the main loop

    // handle second endpoint
    xend = round(x1);
    yend = y1 + gradient * (xend as f32 - x1);
    xgap = fpart(x1 + 0.5);
    let xpxl2 = xend; //this will be used in the main loop
    let ypxl2 = ipart(yend);

    if steep {
        plot(ypxl2, xpxl2, rfpart(yend) * xgap);
        plot(ypxl2 + 1, xpxl2, fpart(yend) * xgap);
    } else {
        plot(xpxl2, ypxl2, rfpart(yend) * xgap);
        plot(xpxl2, ypxl2 + 1, fpart(yend) * xgap);
    }

    // main loop
    if steep {
        for x in (xpxl1 + 1)..=(xpxl2 - 1) {
            plot(ipart(intery), x, rfpart(intery));
            plot(ipart(intery) + 1, x, fpart(intery));
            intery = intery + gradient;
        }
    } else {
        for x in xpxl1 + 1..=xpxl2 - 1 {
            plot(x, ipart(intery), rfpart(intery));
            plot(x, ipart(intery) + 1, fpart(intery));
            intery = intery + gradient;
        }
    }
}
