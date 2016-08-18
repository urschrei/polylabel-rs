use std::cmp::Ordering;
use std::collections::BinaryHeap;

extern crate num;
use self::num::{Float, FromPrimitive};
use self::num::pow::pow;

extern crate geo;
use self::geo::{Point, Polygon};
use self::geo::algorithm::boundingbox::BoundingBox;
use self::geo::algorithm::centroid::Centroid;
use self::geo::algorithm::contains::Contains;

use std::f64::consts::SQRT_2;

/// A helper struct for `polylabel`
/// We're defining it out here because `#[derive]` doesn't work inside functions
#[derive(PartialEq, Debug)]
struct Cell<T>
    where T: Float
{
    x: T, // Centroid x
    y: T, // Centroid y
    h: T, // Half the cell size
    distance: T, // Distance from cell centroid to polygon
    max_distance: T, // Max distance to polygon within a cell
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


// We're going to use this struct as input for a minimum priority queue
// We need this for efficient point-to-polygon distance
#[derive(PartialEq, Debug)]
struct Mindist<T>
    where T: Float
{
    distance: T, // Distance from cell centroid to polygon
}
// These impls give us a min-heap when used with BinaryHeap
impl<T> Ord for Mindist<T>
    where T: Float
{
    fn cmp(&self, other: &Mindist<T>) -> std::cmp::Ordering {
        other.distance.partial_cmp(&self.distance).unwrap()
    }
}
impl<T> PartialOrd for Mindist<T>
    where T: Float
{
    fn partial_cmp(&self, other: &Mindist<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Eq for Mindist<T> where T: Float {}

// Signed distance from a Cell's centroid to a Polygon's outline
// Returned value is negative if the point is outside the polygon's exterior ring
fn signed_distance<T>(x: &T, y: &T, polygon: &Polygon<T>) -> T
    where T: Float
{
    let inside = polygon.contains(&Point::new(*x, *y));
    let distance = point_polygon_distance(x, y, polygon);
    if inside {
        distance
    } else {
        -distance
    }
}

// Minimum distance from a Point to a Polygon
fn point_polygon_distance<T>(x: &T, y: &T, polygon: &Polygon<T>) -> T
    where T: Float
{
    // minimum priority queue
    let mut dist_queue: BinaryHeap<Mindist<T>> = BinaryHeap::new();
    // get exterior ring
    let exterior = &polygon.0;
    // exterior ring as a LineString
    let ext_ring = &exterior.0;
    for chunk in ext_ring.chunks(2) {
        let dist = match chunk.len() {
            2 => {
                pld(&Point::new(*x, *y),
                    chunk.first().unwrap(),
                    chunk.last().unwrap())
            }
            _ => {
                // final point in an odd-numbered exterior ring
                pld(&Point::new(*x, *y),
                    chunk.first().unwrap(),
                    chunk.first().unwrap())
            }
        };
        dist_queue.push(Mindist { distance: dist });
    }
    dist_queue.pop().unwrap().distance
}

// Return minimum distance between Point and a Line segment
// adapted from http://stackoverflow.com/a/1501725/416626
fn pld<T>(point: &Point<T>, start: &Point<T>, end: &Point<T>) -> T
    where T: Float
{
    // line segment distance squared
    let l2 = pow(start.x() - end.x(), 2) + pow(start.y() - end.y(), 2);
    // start == end case
    if l2 == T::zero() { return pow(point.x() - start.x(), 2) + pow(point.y() - start.y(), 2) }
    // Consider the line extending the segment, parameterized as start + t (end - start)
    // We find the projection of the point onto the line
    // This falls where t = [(point - start) . (end - start)] / |end - start|^2, where . is the dot product
    let t = ((point.x() - start.x()) * (end.x() - start.x()) + (point.y() - start.y()) * (end.y() - start.y())) / l2;
    // We clamp t from [0.0, 1.0] to handle points outside the segment start, end
    if t < T::zero() { return (pow(point.x() - start.x(), 2) + pow(point.y() - start.y(), 2)).sqrt() }
    if t > T::one() { return (pow(point.x() - end.x(), 2) + pow(point.y() - end.y(), 2)).sqrt() }
    let projected = Point::new(
        start.x() + t * (end.x() - start.x()),
        start.y() + t * (end.y() - start.y())
    );
    (pow(point.x() - projected.x(), 2) + pow(point.y() - projected.y(), 2)).sqrt()
}

// Calculate ideal label position
fn polylabel<T>(polygon: &Polygon<T>, tolerance: &T) -> Point<T>
    where T: Float + FromPrimitive
{
    let centroid = polygon.centroid().unwrap();
    let bbox = polygon.bbox().unwrap();
    let cell_size = (bbox.xmax - bbox.xmin).min(bbox.ymax - bbox.ymin);
    let mut h: T = cell_size / num::cast(2.0).unwrap();
    let distance: T = signed_distance(&centroid.x(), &centroid.y(), polygon);
    let max_distance: T = distance + h * num::cast(SQRT_2).unwrap();
    // Minimum priority queue
    let mut cell_queue: BinaryHeap<Cell<T>> = BinaryHeap::new();
    let mut best_cell = Cell {
        x: centroid.x(),
        y: centroid.y(),
        h: num::cast(0.0).unwrap(),
        distance: distance,
        max_distance: max_distance,
    };
    // Build a regular square grid, which covers the Polygon
    let mut x = bbox.xmin;
    let mut y;
    while x < bbox.xmax {
        y = bbox.ymin;
        while y < bbox.ymax {
            let latest_dist = signed_distance(&(x + h), &(y + h), polygon);
            cell_queue.push(Cell {
                x: x + h,
                y: y + h,
                h: num::cast(0.0).unwrap(),
                distance: latest_dist,
                max_distance: latest_dist + h * num::cast(SQRT_2).unwrap(),
            });
            y = y + cell_size;
        }
        x = x + cell_size;
    }
    // Pop items off the queue
    while !cell_queue.is_empty() {
        let cell = cell_queue.pop().unwrap();
        h = cell.h / num::cast(2.0).unwrap();
        // Update the best cell if we find a better one
        if cell.distance > best_cell.distance {
            best_cell.x = cell.x;
            best_cell.y = cell.y;
            best_cell.h = cell.h;
            best_cell.distance = cell.distance;
            best_cell.max_distance = cell.max_distance;
        }
        // Bail out of this loop if we can't find a better solution
        if cell.max_distance - best_cell.distance <= *tolerance {
            continue;
        }
        // Otherwise, split the cell into quadrants, and push onto queue
        let mut new_dist = signed_distance(&(cell.x - h), &(cell.y - h), polygon);
        cell_queue.push(Cell {
            x: cell.x - h,
            y: cell.y - h,
            h: h,
            distance: new_dist,
            max_distance: new_dist + h * num::cast(SQRT_2).unwrap(),
        });
        new_dist = signed_distance(&(cell.x + h), &(cell.y - h), polygon);
        cell_queue.push(Cell {
            x: cell.x + h,
            y: cell.y - h,
            h: h,
            distance: new_dist,
            max_distance: new_dist + h * num::cast(SQRT_2).unwrap(),
        });
        new_dist = signed_distance(&(cell.x - h), &(cell.y + h), polygon);
        cell_queue.push(Cell {
            x: cell.x - h,
            y: cell.y + h,
            h: h,
            distance: new_dist,
            max_distance: new_dist + h * num::cast(SQRT_2).unwrap(),
        });
        new_dist = signed_distance(&(cell.x + h), &(cell.y + h), polygon);
        cell_queue.push(Cell {
            x: cell.x + h,
            y: cell.y + h,
            h: h,
            distance: new_dist,
            max_distance: new_dist + h * num::cast(SQRT_2).unwrap(),
        });
    }
    // return best_cell centroid coordinates here
    Point::new(best_cell.x, best_cell.y)
}

#[cfg(test)]
mod tests {
    use std::collections::BinaryHeap;
    use super::{polylabel, point_polygon_distance, pld, Cell, Mindist};
    extern crate geo;
    use geo::{Point, Polygon, LineString};
    #[test]
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
        let res = polylabel(&poly, &10.0);
        assert_eq!(res, Point::new(59.35615556364569, 121.8391962974644));
    }
    #[test]
    fn polygon_distance_test() {
        let coords = vec![
            (5., 1.),
            (4., 2.),
            (4., 3.),
            (5., 4.),
            (6., 4.),
            (7., 3.),
            (7., 2.),
            (6., 1.)
        ];
        let ls = LineString(coords.iter().map(|e| Point::new(e.0, e.1)).collect());
        let poly = Polygon(ls, vec![]);
        let dist = point_polygon_distance(&0.0, &8.0, &poly);
        // result from Shapely
        assert_eq!(dist, 6.363961030678928);

    }
    #[test]
    fn point_line_distance_test() {
        let o1 = Point::new(8.0, 0.0);
        let o2 = Point::new(5.5, 0.0);
        let o3 = Point::new(5.0, 0.0);
        let o4 = Point::new(4.5, 1.5);

        let p1 = Point::new(7.2, 2.0);
        let p2 = Point::new(6.0, 1.0);

        let dist = pld(&o1, &p1, &p2);
        let dist2 = pld(&o2, &p1, &p2);
        let dist3 = pld(&o3, &p1, &p2);
        let dist4 = pld(&o4, &p1, &p2);
        // Result agrees with Shapely
        assert_eq!(dist, 2.0485900789263356);
        assert_eq!(dist2, 1.118033988749895);
        assert_eq!(dist3, 1.4142135623730951);
        assert_eq!(dist4, 1.5811388300841898);
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
    #[test]
    // Is our minimum distance queue behaving as it should?
    fn test_dist_queue() {
        let a = Mindist { distance: 4.0 };
        let b = Mindist { distance: 1.0 };
        let c = Mindist { distance: 6.0 };
        let mut v = vec![];
        v.push(a);
        v.push(b);
        v.push(c);
        let mut q = BinaryHeap::from(v);
        assert_eq!(q.pop().unwrap().distance, 1.0);
        assert_eq!(q.pop().unwrap().distance, 4.0);
        assert_eq!(q.pop().unwrap().distance, 6.0);
    }
}
