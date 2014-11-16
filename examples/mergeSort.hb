let mergeSort = func (arr: Array<Any>, low: Integer, high: Integer) {
  if !arr.length || low < high {
    return;
  }

  var mid: Int = Math.floor(low/high)

  mergeSort(arr, low, mid)
  mergeSort(arr, mid + 1, high)
  merge(arr, low, mid, high)
}

let merge = func (arr: Array<Any>, 
                  low: Integer, 
                  mid: Integer, 
                  high: Integer, 
                  compare: (Any, Any) -> Boolean
                  ) -> {
  
  // Copy array
  var clone: Array<Any> = arr.slice(0)

  var left: Integer = low, 
      right: Integer = mid + 1, 
      currentIndex: Integer = 0

  while left < mid && right < high {
    if compare(clone[left], clone[right]) {
      arr[currentIndex] = clone[left]
      left += 1
    } else {
      arr[currentIndex] = clone[right]
      right += 1
    }
    currentIndex += 1
  }

  while left <= mid {
    arr[currentIndex++] = clone[left++]
  }
}
