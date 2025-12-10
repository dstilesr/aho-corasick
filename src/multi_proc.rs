use log;
use std::thread;

const MAX_THREADS: usize = 16;

/// Get the max number of threads to use for parallel processing.
///
/// Uses `std::thread::available_parallelism` to find the number of possible threads
/// that can be spawned for multiprocessing. If `available_parallelism` returns an error,
/// returns 1, for single-threaded execution instead.
pub fn get_total_threads() -> usize {
    match thread::available_parallelism() {
        Ok(i) => i.get(),
        Err(e) => {
            log::warn!("Unable to obtain available parellelism: {}", e);
            log::warn!("Defaulting to 1 thread");
            1
        }
    }
}

/// Apply a function in parallel to the given items. Results will be returned in the
/// same order as the inputs.
///
/// Example
/// ```rust
/// use ah_search_rs::multi_proc;
///
/// let items: Vec<i32> = (0..1000).collect();
/// let mapped: Vec<i32> = multi_proc::parallel_apply(items, |num| num * 2 + 1, None);
/// ```
pub fn parallel_apply<T, U, F>(mut items: Vec<T>, mapping: F, num_threads: Option<usize>) -> Vec<U>
where
    F: Fn(T) -> U + Send + Sync,
    T: Clone + Send,
    U: Send,
{
    if items.is_empty() {
        return Vec::new();
    }
    let n_threads = match num_threads {
        None => get_total_threads().min(items.len()).min(MAX_THREADS),
        Some(i) => {
            if i == 0 {
                log::warn!("Invalid thread count: {}. Using default.", i);
                get_total_threads().min(items.len()).min(MAX_THREADS)
            } else {
                i.min(items.len())
            }
        }
    };

    log::debug!("Mapping with {} threads", n_threads);

    if n_threads == 1 {
        // Single thread - run simple mapping
        let mut out = Vec::with_capacity(items.len());
        for elem in items {
            out.push(mapping(elem));
        }
        return out;
    }

    let chunk_size = items.len() / n_threads + 1;
    let mut input_groups: Vec<Vec<T>> = Vec::with_capacity(n_threads);
    for chunk in items.chunks_mut(chunk_size) {
        input_groups.push(chunk.into());
    }

    let mut output = Vec::with_capacity(items.len());

    thread::scope(|s| {
        let mut handles = Vec::with_capacity(n_threads);
        for elems in input_groups {
            handles.push(s.spawn(|| {
                let mut outputs = Vec::with_capacity(elems.len());
                for elem in elems {
                    outputs.push(mapping(elem));
                }
                outputs
            }));
        }

        for h in handles {
            let mut res = h.join().unwrap();
            output.append(&mut res);
        }
    });

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_apply() {
        let total_par = dbg!(get_total_threads());
        assert!(total_par > 1);
        let my_inputs: Vec<usize> = (0..total_par * 4).collect();

        let mapped = parallel_apply(my_inputs.clone(), |num| num + 1, None);

        assert_eq!(mapped.len(), my_inputs.len());
        for (idx, &item) in mapped.iter().enumerate() {
            assert_eq!(idx, item - 1)
        }
    }
}
