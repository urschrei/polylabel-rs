extern crate num;
use self::num::{Float, FromPrimitive};

extern crate geo;
use self::geo::{Point, Polygon, LineString};
use self::geo::algorithm::boundingbox::BoundingBox;
use self::geo::algorithm::centroid::Centroid;
use self::geo::algorithm::distance::Distance;
use self::geo::algorithm::contains::Contains;


// use std::f64;
// use std::cmp;
use std::collections::BinaryHeap;

/// A helper struct for `polylabel`
/// We're defining it out here because `#[derive]` doesn't work inside functions
#[derive(PartialOrd, PartialEq, Debug)]
struct Cell<T>
    where T: Float
{
    x: T, // cell centre x
    y: T, // cell centre y
    h: T, // half the cell size
    distance: T, // distance from cell centroid to polygon
    max_distance: T, // max distance to polygon within a cell
}

// Signed distance from a Cell's centroid to a Polygon's outline
// Returned value is negative if the point is outside the polygon's exterior ring
fn signed_distance<T>(x: &T, y: &T, polygon: &Polygon<T>) -> T
    where T: Float
{
    let ref ls = polygon.0;
    let ref points = ls.0;
    let inside = polygon.contains(&Point::new(*x, *y));
    let distance = pld(&Point::new(*x, *y), &points[0], &points.last().unwrap());
    if inside { distance } else { -distance }
}

impl<T> Ord for Cell<T>
    where T: Float
{
    fn cmp(&self, other: &Cell<T>) -> std::cmp::Ordering {
        // self.partial_cmp(other).unwrap()
        self.max_distance.partial_cmp(&other.max_distance).unwrap()
    }
}
impl<T> Eq for Cell<T> where T: Float {}

// perpendicular distance from a point to a line
fn pld<T>(point: &Point<T>, start: &Point<T>, end: &Point<T>) -> T
    where T: Float
{
    if start == end {
        point.distance(start)
    } else {
        let numerator = ((end.x() - start.x()) * (start.y() - point.y()) -
                         (start.x() - point.x()) * (end.y() - start.y()))
            .abs();
        let denominator = start.distance(end);
        numerator / denominator
    }
}

// https://github.com/mapbox/polylabel/blob/master/index.js#L7-L71
fn polylabel<T>(polygon: &Polygon<T>, tolerance: &T) -> Point<T>
    where T: Float + FromPrimitive
{
    let centroid = polygon.centroid().unwrap();
    let bbox = polygon.bbox().unwrap();
    let cell_size = (bbox.xmax - bbox.xmin).min(bbox.ymax - bbox.ymin);
    let mut h: T = cell_size / num::cast(2.0).unwrap();
    let distance: T = signed_distance(&centroid.x(), &centroid.y(), &polygon);
    let max_distance: T = distance + h * num::cast(1.4142135623730951).unwrap();
    let mut cell_queue: BinaryHeap<Cell<T>> = BinaryHeap::new();
    let mut best_cell = Cell {
        x: centroid.x(),
        y: centroid.y(),
        h: num::cast(0.0).unwrap(),
        distance: distance,
        max_distance: max_distance,
    };
    let mut x = bbox.xmin;
    let mut y;
    while x < bbox.xmax {
        y = bbox.ymin;
        while y < bbox.ymax {
            let latest_dist = signed_distance(&(x + h), &(y + h), &polygon);
            cell_queue.push(Cell {
                x: x + h,
                y: y + h,
                h: num::cast(0.0).unwrap(),
                distance: latest_dist,
                max_distance: latest_dist + h * num::cast(1.4142135623730951).unwrap(),
            });
            y = y + cell_size;
        }
        x = x + cell_size;
    }
    // now start popping items off the queue
    while !cell_queue.is_empty() {
        let cell = cell_queue.pop().unwrap();
        h = cell.h / num::cast(2.0).unwrap();
        // update the best cell if we find a better one
        if cell.distance > best_cell.distance {
            best_cell.x = cell.x;
            best_cell.y = cell.y;
            best_cell.distance = cell.distance;
        }
        if cell.max_distance - best_cell.distance <= *tolerance {
            continue;
        }
        let d1 = signed_distance(&(cell.x - h), &(cell.y - h), &polygon);
        cell_queue.push(Cell {
                x: cell.x - h,
                y: cell.y - h,
                h: h,
                distance: d1,
                max_distance: d1 + h * num::cast(1.4142135623730951).unwrap(),
        });
        let d2 = signed_distance(&(cell.x + h), &(cell.y - h), &polygon);
        cell_queue.push(Cell {
                x: cell.x + h,
                y: cell.y - h,
                h: h,
                distance: d2,
                max_distance: d2 + h * num::cast(1.4142135623730951).unwrap(),
        });
        let d3 = signed_distance(&(cell.x - h), &(cell.y + h), &polygon);
        cell_queue.push(Cell {
                x: cell.x - h,
                y: cell.y + h,
                h: h,
                distance: d3,
                max_distance: d3 + h * num::cast(1.4142135623730951).unwrap(),
        });
        let d4 = signed_distance(&(cell.x + h), &(cell.y + h), &polygon);
        cell_queue.push(Cell {
                x: cell.x + h,
                y: cell.y + h,
                h: h,
                distance: d4,
                max_distance: d4 + h * num::cast(1.4142135623730951).unwrap(),
        });
    }
    // return best_cell centroid coordinates here
    Point::new(best_cell.x, best_cell.y)
}

#[cfg(test)]
mod tests {
    use super::polylabel;
    extern crate geo;
    use geo::{Point, Polygon, LineString};
    #[test]
    fn test_polylabel() {
        let coords = vec![
            (-75.57274028771249, 110.01960141091608),
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
            (-75.57274028771249, 110.01960141091608)
        ];
        let ls = LineString(coords.iter().map(|e| { Point::new(e.0, e.1) }).collect());
        let poly = Polygon(ls, vec![]);
        let res = polylabel(&poly, &10.0);
        // hmm
        assert_eq!(res, Point::new(59.35615556364569, 121.8391962974644));
    }
}
