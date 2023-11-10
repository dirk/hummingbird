/// Compare two `Vec`s for equality using a custom comparator function to
/// check equality of each element.
pub fn vecs_equal<T, F: Fn(&T, &T) -> bool>(left: &Vec<T>, right: &Vec<T>, cmp: F) -> bool {
    if left.len() != right.len() {
        return false;
    }
    for (index, left_element) in left.iter().enumerate() {
        let right_element = &right[index];
        if !cmp(left_element, right_element) {
            return false;
        }
    }
    true
}
