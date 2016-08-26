#![doc(html_logo_url = "https://cdn.rawgit.com/urschrei/polylabel-rs/5ab07d193f61bb0e16338a6d19a08ba32f153ddb/ell.svg",
       html_root_url = "https://urschrei.github.io/polylabel-rs/")]
//! This crate provides a Rust implementation of the [Polylabel](https://github.com/mapbox/polylabel) algorithm
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::slice;
use std::mem;

extern crate libc;
use self::libc::{c_void, c_double, size_t};

extern crate num;
use self::num::{Float, FromPrimitive, ToPrimitive};
use self::num::pow::pow;

extern crate geo;
use self::geo::{Point, Polygon, LineString};
use self::geo::algorithm::boundingbox::BoundingBox;
use self::geo::algorithm::distance::Distance;
use self::geo::algorithm::centroid::Centroid;
use self::geo::algorithm::contains::Contains;

// A helper struct for `polylabel`
#[derive(PartialEq, Debug)]
struct Cell<T>
    where T: Float
{
    // Centroid coordinates
    x: T,
    y: T,
    // Half the cell size
    h: T,
    // Distance from centroid to polygon
    distance: T,
    // Maximum distance to polygon within a cell
    max_distance: T,
}

// These impls give us a min-heap when used with BinaryHeap
impl<T> Ord for Cell<T>
    where T: Float
{
    fn cmp(&self, other: &Cell<T>) -> std::cmp::Ordering {
        other.max_distance.partial_cmp(&self.max_distance).unwrap()
    }
}
impl<T> PartialOrd for Cell<T>
    where T: Float
{
    fn partial_cmp(&self, other: &Cell<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Eq for Cell<T> where T: Float {}

// Signed distance from a Cell's centroid to a Polygon's outline
// Returned value is negative if the point is outside the polygon's exterior ring
fn signed_distance<T>(x: &T, y: &T, polygon: &Polygon<T>) -> T
    where T: Float
{
    let point = Point::new(*x, *y);
    let inside = polygon.contains(&point);
    // Use LineString distance, because Polygon distance returns 0.0 for inside
    let distance = point.distance(&polygon.0);
    if inside {
        distance
    } else {
        -distance
    }
}

// Add a new quadtree node to the minimum priority queue
fn add_quad<T>(mpq: &mut BinaryHeap<Cell<T>>, cell: &Cell<T>, nh: &T, polygon: &Polygon<T>)
    where T: Float
{
    let two = T::one() + T::one();
    // 1
    let mut new_dist = signed_distance(&(cell.x - *nh), &(cell.y - *nh), polygon);
    mpq.push(Cell {
        x: cell.x - *nh,
        y: cell.y - *nh,
        h: *nh,
        distance: new_dist,
        max_distance: new_dist + *nh * two.sqrt(),
    });
    // 2
    new_dist = signed_distance(&(cell.x + *nh), &(cell.y - *nh), polygon);
    mpq.push(Cell {
        x: cell.x + *nh,
        y: cell.y - *nh,
        h: *nh,
        distance: new_dist,
        max_distance: new_dist + *nh * two.sqrt(),
    });
    // 3
    new_dist = signed_distance(&(cell.x - *nh), &(cell.y + *nh), polygon);
    mpq.push(Cell {
        x: cell.x - *nh,
        y: cell.y + *nh,
        h: *nh,
        distance: new_dist,
        max_distance: new_dist + *nh * two.sqrt(),
    });
    // 4
    new_dist = signed_distance(&(cell.x + *nh), &(cell.y + *nh), polygon);
    mpq.push(Cell {
        x: cell.x + *nh,
        y: cell.y + *nh,
        h: *nh,
        distance: new_dist,
        max_distance: new_dist + *nh * two.sqrt(),
    });
}

/// Wrapper for a void pointer to a sequence of Arrays
/// Used for inner rings
#[repr(C)]
pub struct WrapperArray {
    pub data: *const Array,
    pub len: size_t,
}

/// Outer polygon rings
///
/// Can be:
///
/// - `Vec<[c_double; 2]>` (exterior ring)
/// - `Vec<Vec<[c_double: 2]>>` (interior rings)
#[repr(C)]
pub struct Array {
    pub data: *const c_void,
    pub len: size_t,
}

/// Optimum Polygon label position
#[repr(C)]
pub struct Position {
    pub x_pos: c_double,
    pub y_pos: c_double,
}

// convert a Polylabel result Point into values that can be sent across the FFI boundary
impl<T> From<Point<T>> for Position
    where T: Float
{
    fn from(point: Point<T>) -> Position {
        Position {
            x_pos: point.x().to_f64().unwrap() as c_double,
            y_pos: point.y().to_f64().unwrap() as c_double,
        }
    }
}

/// FFI access to the [`polylabel`](fn.polylabel.html) function
///
/// Accepts three arguments: an exterior ring [`Array`](struct.Array.html), an interior rings [`Array`](struct.Array.html), and a tolerance.
#[no_mangle]
pub extern "C" fn polylabel_ffi(outer: Array,
                                inners: WrapperArray,
                                tolerance: c_double)
                                -> Position {
    let exterior: Vec<[f64; 2]> =
        unsafe { slice::from_raw_parts(outer.data as *mut [c_double; 2], outer.len).to_vec() };
    let interior: Vec<Vec<[f64; 2]>> = reconstitute2(inners);
    let ls_ext = LineString(exterior.iter().map(|e| Point::new(e[0], e[1])).collect());
    let ls_int: Vec<LineString<c_double>> = interior.iter()
        .map(|vec| LineString(vec.iter().map(|e| Point::new(e[0], e[1])).collect()))
        .collect();

    let poly = Polygon(ls_ext, ls_int);
    polylabel(&poly, &tolerance).into()
}

/// Calculate a Polygon's ideal label position by calculating its ✨pole of inaccessibility✨
///
/// The calculation uses an [iterative grid-based algorithm](https://github.com/mapbox/polylabel#how-the-algorithm-works).
///
/// # Examples
///
/// ```
/// use polylabel::polylabel;
/// extern crate geo;
/// use self::geo::{Point, LineString, Polygon};
///
/// // An approximate `L` shape
/// let coords = vec![
///    (0.0, 0.0),
///    (4.0, 0.0),
///    (4.0, 1.0),
///    (1.0, 1.0),
///    (1.0, 4.0),
///    (0.0, 4.0),
///    (0.0, 0.0)];
///
/// let ls = LineString(coords.iter().map(|e| Point::new(e.0, e.1)).collect());
/// let poly = Polygon(ls, vec![]);
///
/// // Its centroid lies outside the polygon
/// assert_eq!(poly.centroid(), Point::new(1.3571428571428572, 1.3571428571428572));
///
/// let label_position = polylabel(&poly, &1.0);
/// // Optimum label position is inside the polygon
/// assert_eq!(label_position, Point::new(0.5625, 0.5625));
/// ```
///
pub fn polylabel<T>(polygon: &Polygon<T>, tolerance: &T) -> Point<T>
    where T: Float + FromPrimitive
{
    let two = T::one() + T::one();
    // Initial best cell values
    let centroid = polygon.centroid().unwrap();
    let bbox = polygon.bbox().unwrap();
    let cell_size = (bbox.xmax - bbox.xmin).min(bbox.ymax - bbox.ymin);
    let mut h: T = cell_size / two;
    let distance: T = signed_distance(&centroid.x(), &centroid.y(), polygon);
    let max_distance: T = distance + T::zero() * two.sqrt();

    let mut best_cell = Cell {
        x: centroid.x(),
        y: centroid.y(),
        h: T::zero(),
        distance: distance,
        max_distance: max_distance,
    };
    // Minimum priority queue
    let mut cell_queue: BinaryHeap<Cell<T>> = BinaryHeap::new();
    // Build an initial quadtree node, which covers the Polygon
    let mut x = bbox.xmin;
    let mut y;
    while x < bbox.xmax {
        y = bbox.ymin;
        while y < bbox.ymax {
            let latest_dist = signed_distance(&(x + h), &(y + h), polygon);
            cell_queue.push(Cell {
                x: x + h,
                y: y + h,
                h: h,
                distance: latest_dist,
                max_distance: latest_dist + h * two.sqrt(),
            });
            y = y + cell_size;
        }
        x = x + cell_size;
    }
    // Now try to find better solutions
    while !cell_queue.is_empty() {
        let cell = cell_queue.pop().unwrap();
        // Update the best cell if we find a cell with greater distance
        if cell.distance > best_cell.distance {
            best_cell.x = cell.x;
            best_cell.y = cell.y;
            best_cell.h = cell.h;
            best_cell.distance = cell.distance;
            best_cell.max_distance = cell.max_distance;
        }
        // Bail out of this iteration if we can't find a better solution
        if cell.max_distance - best_cell.distance <= *tolerance {
            continue;
        }
        // Otherwise, add a new quadtree node and start again
        h = cell.h / two;
        add_quad(&mut cell_queue, &cell, &h, polygon);
    }
    // We've exhausted the queue, so return the best solution we've found
    Point::new(best_cell.x, best_cell.y)
}

fn gen_array(v: Vec<[f64; 2]>) -> Array {
    let array = Array {
        data: v.as_ptr() as *const c_void,
        len: v.len() as size_t,
    };
    mem::forget(v);
    array
}

fn gen_array2(v: Vec<Vec<[f64; 2]>>) -> WrapperArray {
    let converted: Vec<Array> = v.into_iter().map(|x| gen_array(x)).collect();
    let array2 = WrapperArray {
        data: converted.as_ptr() as *const Array,
        len: converted.len() as size_t,
    };
    mem::forget(converted);
    array2
}

fn reconstitute(arr: &Array) -> Vec<[f64; 2]> {
    unsafe { slice::from_raw_parts(arr.data as *mut [f64; 2], arr.len).to_vec() }
}

fn reconstitute2(arr: WrapperArray) -> Vec<Vec<[f64; 2]>> {
    let arrays = unsafe { slice::from_raw_parts(arr.data as *mut Array, arr.len) };
    arrays.into_iter().map(|x| reconstitute(x)).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BinaryHeap;
    use super::{polylabel, Cell, gen_array2, reconstitute2};
    extern crate geo;
    use geo::{Point, Polygon, LineString};
    use geo::contains::Contains;
    #[test]
    fn test_array() {
        let i_a = vec![[0.5, 0.5], [1.0, 1.0], [1.5, 0.5]];
        let i_b = vec![[0.55, 0.55], [0.8, 0.8], [1.2, 0.55]];
        let inners = vec![i_a, i_b];
        let array = gen_array2(inners);
        let rec_inners = reconstitute2(array);
        assert_eq!(rec_inners[0][2], [1.5, 0.5])
    }
    #[test]
    // polygons are those used in Shapely's tests
    fn test_polylabel() {
        let coords = vec![(-75.57274028771249, 110.01960141091608),
                          (-47.01425001453319, 224.2535625036333),
                          (-44.1986052400443, 233.56520178444188),
                          (-40.491516848197264, 242.55919851821028),
                          (-35.928066677809895, 251.1504384831045),
                          (-30.55144070299677, 259.2576189250935),
                          (-24.412520344941953, 266.8040179623472),
                          (-17.56940095820731, 273.7182206387879),
                          (-10.086842048356445, 279.93479475582495),
                          (-2.0356544237308825, 285.394910086574),
                          (6.507969918963688, 290.0468951126591),
                          (15.463178833668529, 293.8467260149487),
                          (24.745225165064543, 296.758443290685),
                          (34.26626874888323, 298.7544920543751),
                          (43.93620768274765, 299.8159828020204),
                          (53.66353100085455, 299.9328701709592),
                          (63.35618468325523, 299.10404800363494),
                          (72.92244280430123, 297.33735981566144),
                          (82.27177557618647, 294.64952456911897),
                          (91.31570607290114, 291.0659784535237),
                          (99.96864752703414, 286.6206341717666),
                          (108.14871327570971, 281.355560009008),
                          (115.77849169077639, 275.3205817216414),
                          (122.78577875973701, 268.57281101383126),
                          (129.10426138467784, 261.17610506386103),
                          (134.67414493283295, 253.20046221503722),
                          (139.44271909999156, 244.72135954999587),
                          (189.4427190999916, 144.72135954999578),
                          (193.40316487178438, 135.7190256296874),
                          (196.46014848027284, 126.37119176608674),
                          (198.5841005952538, 116.76827688896145),
                          (199.75447683394128, 107.00316725394137),
                          (199.959956480919, 97.1703179802708),
                          (199.19855199019082, 87.36483941339068),
                          (197.47762821014587, 77.68157714970485),
                          (194.8138311454814, 68.21419462218802),
                          (191.23292694514913, 59.05426712072333),
                          (186.76955267374814, 50.29039601045239),
                          (181.46688127708177, 42.007351716050565),
                          (175.37620398257155, 34.28525376159651),
                          (168.5564341738489, 27.198795797276006),
                          (161.07353753840516, 20.81652310901589),
                          (152.99989400031805, 15.200169599491232),
                          (98.33653286253586, -18.964431111622638),
                          (97.01425001453319, -24.253562503633297),
                          (94.16983504461093, -33.64583432864707),
                          (90.41851308474087, -42.71407837639184),
                          (85.79641141607766, -51.37096249948156),
                          (80.34804340438832, -59.53311617147662),
                          (74.12587981200636, -67.12193339062866),
                          (67.189843475707, -74.06432969864774),
                          (59.606732217031976, -80.2934460239878),
                          (51.44957554275259, -85.74929257125446),
                          (42.79693133079759, -90.37932655572841),
                          (33.73212927494458, -94.13895821910516),
                          (24.342468374316272, -96.99198025324264),
                          (14.718376196296493, -98.91091649633165),
                          (4.952538009623515, -99.87728654335396),
                          (-4.860995825414805, -99.88178372248515),
                          (-14.627715613363762, -98.92436472343178),
                          (-17.953756809330994, -98.26435835897965),
                          (-53.64820903700594, -226.76438637860946),
                          (-56.7355378616229, -236.07963555856995),
                          (-60.72105444017349, -245.0474181249662),
                          (-65.5663760693013, -253.58136942939535),
                          (-71.22483965299563, -261.59930285566344),
                          (-77.64195109371464, -269.02400132182726),
                          (-84.75591010033425, -275.7839609229046),
                          (-92.49820535873518, -281.81407955256725),
                          (-100.79427433320987, -287.05628387201347),
                          (-109.56422134444159, -291.46008858796654),
                          (-118.72358700857137, -294.98308265364733),
                          (-128.18416162723517, -297.59133771033885),
                          (-137.85483469517902, -299.2597348360279),
                          (-147.64247234423098, -299.9722064543555),
                          (-157.4528142733637, -299.72189107416057),
                          (-167.19138152692128, -298.5111993693906),
                          (-176.76438637860946, -296.3517909629941),
                          (-186.0796355585698, -293.26446213837716),
                          (-195.04741812496607, -289.2789455598266),
                          (-203.58136942939524, -284.4336239306988),
                          (-211.59930285566332, -278.7751603470045),
                          (-219.02400132182714, -272.3580489062855),
                          (-225.78396092290453, -265.2440898996658),
                          (-231.8140795525672, -257.50179464126495),
                          (-237.0562838720134, -249.20572566679022),
                          (-241.46008858796648, -240.4357786555585),
                          (-244.98308265364727, -231.27641299142869),
                          (-247.59133771033882, -221.8158383727649),
                          (-249.25973483602792, -212.145165304821),
                          (-249.97220645435553, -202.35752765576902),
                          (-249.72189107416057, -192.54718572663626),
                          (-248.51119936939062, -182.8086184730787),
                          (-246.35179096299407, -173.23561362139054),
                          (-196.35179096299407, 6.7643863786094585),
                          (-193.32576660256726, 15.920764023655508),
                          (-189.43184924301974, 24.74309266215056),
                          (-184.7062507874361, 33.14932810051302),
                          (-179.19291744665992, 41.0612956486063),
                          (-172.94312105678188, 48.40541711367358),
                          (-166.01498227118805, 55.11339504865113),
                          (-158.47293006129595, 61.12284789161923),
                          (-150.3871025524086, 66.37789008984335),
                          (-75.57274028771249, 110.01960141091608)];
        let ls = LineString(coords.iter().map(|e| Point::new(e.0, e.1)).collect());
        let poly = Polygon(ls, vec![]);
        let res = polylabel(&poly, &10.000);
        assert_eq!(res, Point::new(59.35615556364569, 121.83919629746435));
    }
    #[test]
    // does a concave polygon contain the calculated point?
    // relevant because the centroid lies outside the polygon in this case
    fn test_concave() {
        let coords = vec![(0.0, -100.0),
                          (-9.801714032956664, -99.51847266721963),
                          (-19.509032201613387, -98.07852804032294),
                          (-29.02846772544675, -95.69403357322072),
                          (-38.268343236509445, -92.38795325112848),
                          (-47.13967368260018, -88.19212643483529),
                          (-55.5570233019606, -83.14696123025428),
                          (-63.439328416364795, -77.30104533627349),
                          (-70.710678118655, -70.71067811865453),
                          (-77.30104533627389, -63.439328416364326),
                          (-83.14696123025468, -55.55702330196001),
                          (-88.1921264348356, -47.13967368259957),
                          (-92.38795325112875, -38.2683432365088),
                          (-95.69403357322092, -29.028467725446088),
                          (-98.07852804032306, -19.509032201612705),
                          (-99.5184726672197, -9.80171403295597),
                          (-100.0, 0.0),
                          (-100.0, 500.0),
                          (-99.51847266721968, 509.80171403295606),
                          (-98.07852804032305, 519.5090322016129),
                          (-95.69403357322088, 529.0284677254463),
                          (-92.38795325112868, 538.2683432365089),
                          (-88.1921264348355, 547.1396736825998),
                          (-83.14696123025453, 555.5570233019603),
                          (-77.3010453362737, 563.4393284163646),
                          (-70.71067811865474, 570.7106781186548),
                          (-63.439328416364525, 577.3010453362738),
                          (-55.55702330196019, 583.1469612302545),
                          (-47.13967368259977, 588.1921264348355),
                          (-38.268343236508976, 592.3879532511287),
                          (-29.028467725446223, 595.6940335732208),
                          (-19.5090322016128, 598.078528040323),
                          (-9.80171403295602, 599.5184726672197),
                          (0.0, 600.0),
                          (500.0, 600.0),
                          (509.8017140329562, 599.5184726672196),
                          (519.509032201613, 598.078528040323),
                          (529.0284677254464, 595.6940335732208),
                          (538.268343236509, 592.3879532511287),
                          (547.1396736825999, 588.1921264348355),
                          (555.5570233019603, 583.1469612302544),
                          (563.4393284163647, 577.3010453362737),
                          (570.7106781186549, 570.7106781186546),
                          (577.3010453362738, 563.4393284163644),
                          (583.1469612302545, 555.5570233019602),
                          (588.1921264348355, 547.1396736825997),
                          (592.3879532511287, 538.2683432365089),
                          (595.6940335732208, 529.0284677254463),
                          (598.078528040323, 519.5090322016129),
                          (599.5184726672197, 509.80171403295606),
                          (600.0, 500.0),
                          (599.5184726672197, 490.19828596704394),
                          (598.078528040323, 480.4909677983872),
                          (595.6940335732208, 470.9715322745538),
                          (592.3879532511287, 461.73165676349106),
                          (588.1921264348355, 452.8603263174003),
                          (583.1469612302545, 444.44297669803984),
                          (577.3010453362738, 436.5606715836355),
                          (570.7106781186548, 429.28932188134524),
                          (563.4393284163646, 422.69895466372634),
                          (555.5570233019603, 416.85303876974547),
                          (547.1396736825998, 411.8078735651645),
                          (538.2683432365089, 407.6120467488713),
                          (529.0284677254463, 404.3059664267791),
                          (519.5090322016127, 401.921471959677),
                          (509.801714032956, 400.4815273327803),
                          (500.0, 400.0),
                          (100.0, 400.0),
                          (100.0, 100.0),
                          (500.0, 100.0),
                          (509.8017140329562, 99.51847266721967),
                          (519.509032201613, 98.07852804032302),
                          (529.0284677254464, 95.69403357322085),
                          (538.268343236509, 92.38795325112864),
                          (547.1396736825999, 88.19212643483544),
                          (555.5570233019603, 83.14696123025446),
                          (563.4393284163647, 77.30104533627363),
                          (570.7106781186549, 70.7106781186547),
                          (577.3010453362738, 63.439328416364496),
                          (583.1469612302545, 55.55702330196017),
                          (588.1921264348355, 47.139673682599714),
                          (592.3879532511287, 38.26834323650893),
                          (595.6940335732208, 29.028467725446205),
                          (598.078528040323, 19.509032201612804),
                          (599.5184726672197, 9.801714032956049),
                          (600.0, 0.0),
                          (599.5184726672197, -9.801714032956049),
                          (598.078528040323, -19.509032201612804),
                          (595.6940335732208, -29.028467725446205),
                          (592.3879532511287, -38.26834323650893),
                          (588.1921264348355, -47.139673682599714),
                          (583.1469612302545, -55.557023301960186),
                          (577.3010453362738, -63.439328416364525),
                          (570.7106781186548, -70.71067811865474),
                          (563.4393284163646, -77.30104533627369),
                          (555.5570233019603, -83.14696123025452),
                          (547.1396736825998, -88.1921264348355),
                          (538.2683432365089, -92.38795325112868),
                          (529.0284677254463, -95.69403357322089),
                          (519.5090322016127, -98.07852804032305),
                          (509.801714032956, -99.5184726672197),
                          (500.0, -100.0),
                          (0.0, -100.0)];

        let ls = LineString(coords.iter().map(|e| Point::new(e.0, e.1)).collect());
        let poly = Polygon(ls, vec![]);
        let res = polylabel(&poly, &1.0);
        assert!(poly.contains(&res));
    }
    #[test]
    fn polygon_l_test() {
        // an L shape
        let coords = vec![(0.0, 0.0), (4.0, 0.0), (4.0, 1.0), (1.0, 1.0), (1.0, 4.0), (0.0, 4.0),
                          (0.0, 0.0)];
        let ls = LineString(coords.iter().map(|e| Point::new(e.0, e.1)).collect());
        let poly = Polygon(ls, vec![]);
        let res = polylabel(&poly, &0.10);
        assert_eq!(res, Point::new(0.5625, 0.5625));
    }
    #[test]
    // Is our minimum priority queue behaving as it should?
    fn test_queue() {
        let a = Cell {
            x: 1.0,
            y: 2.0,
            h: 3.0,
            distance: 4.0,
            max_distance: 8.0,
        };
        let b = Cell {
            x: 1.0,
            y: 2.0,
            h: 3.0,
            distance: 4.0,
            max_distance: 7.0,
        };
        let c = Cell {
            x: 1.0,
            y: 2.0,
            h: 3.0,
            distance: 4.0,
            max_distance: 9.0,
        };
        let mut v = vec![];
        v.push(a);
        v.push(b);
        v.push(c);
        let mut q = BinaryHeap::from(v);
        assert_eq!(q.pop().unwrap().max_distance, 7.0);
        assert_eq!(q.pop().unwrap().max_distance, 8.0);
        assert_eq!(q.pop().unwrap().max_distance, 9.0);
    }
}
