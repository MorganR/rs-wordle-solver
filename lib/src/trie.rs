use radix_trie;
use radix_trie::TrieCommon;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub struct Node<'a, V> {
    pub key: &'a str,
    pub value: &'a V,
}

#[derive(Clone)]
struct StoredKeys<'a> {
    list: Vec<Rc<str>>,
    phantom: PhantomData<&'a str>,
}

impl<'a> StoredKeys<'a> {
    pub fn new() -> StoredKeys<'a> {
        StoredKeys {
            list: vec![Rc::from("")],
            phantom: PhantomData,
        }
    }

    pub fn insert(&mut self, key: &str) {
        self.list.push(Rc::from(key));
    }

    pub fn get_last(&self) -> &'a str {
        let key_ref = self.list.last().unwrap();
        let key_ptr = key_ref.as_ptr();
        unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts::<'a>(
                key_ptr,
                key_ref.as_ref().len(),
            ))
        }
    }
}

/// Implements a Trie, hiding the implementation details from the rest of this library.
#[derive(Clone)]
pub struct Trie<'a, V> {
    keys: StoredKeys<'a>,
    root: radix_trie::Trie<&'a str, V>,
}

impl<'a, V> Trie<'a, V> {
    /// Creates an empty Trie.
    pub fn new() -> Trie<'a, V> {
        Trie {
            keys: StoredKeys::new(),
            root: radix_trie::Trie::new(),
        }
    }

    /// Inserts the given value into the trie using the given key.
    pub fn insert<'b>(&mut self, key: &'b str, value: V) {
        self.keys.insert(key);
        let stored_key: &'a str = self.keys.get_last().as_ref();
        self.root.insert(stored_key, value);
    }

    /// Gets node from the trie with the longest matching prefix.
    ///
    /// For example, if the trie contained the keys: ["ab", "abc", "abcd"], then this would return
    /// as follows:
    ///
    /// * "abd" -> return node for "ab"
    /// * "abc" -> return node for "abc"
    /// * "abcd" -> return node for "abcd"
    /// * "abcde" -> return node for "abcd"
    pub fn get_ancestor<'b>(&'a self, key: &'b str) -> Option<Node<'a, V>> {
        let maybe_prefix_key = self
            .root
            .get_ancestor(key)
            .map(|sub_trie| *sub_trie.key().unwrap());
        maybe_prefix_key.map(|prefix_key| Node {
            key: prefix_key,
            value: self.root.subtrie(prefix_key).unwrap().value().unwrap(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn retrieve_empty() {
        let trie: Trie<u32> = Trie::new();

        assert_eq!(trie.get_ancestor("key"), None);
    }

    #[test]
    fn insert_and_retrieve() {
        let mut trie: Trie<u32> = Trie::new();

        trie.insert("ab", 2);

        let node = trie.get_ancestor("ab").unwrap();
        assert_eq!(node.key, "ab");
        assert_eq!(*node.value, 2);
        assert_eq!(trie.get_ancestor("other"), None);
    }

    #[test]
    fn insert_and_retrieve_prefix() {
        let mut trie: Trie<u32> = Trie::new();

        trie.insert("ab", 2);
        trie.insert("abc", 3);

        let node = trie.get_ancestor("abcd").unwrap();
        assert_eq!(node.key, "abc");
        assert_eq!(*node.value, 3);
    }

    #[test]
    fn insert_and_retrieve_prefix_other_path() {
        let mut trie: Trie<u32> = Trie::new();

        trie.insert("ab", 2);
        trie.insert("abc", 3);

        let node = trie.get_ancestor("abd").unwrap();
        assert_eq!(node.key, "ab");
        assert_eq!(*node.value, 2);
    }
}
