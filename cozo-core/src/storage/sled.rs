/*
 * Copyright 2022, The Cozo Project Authors.
 *
 * This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
 * If a copy of the MPL was not distributed with this file,
 * You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::iter;
use std::path::Path;
use std::sync::Arc;

use fjall::{Database, Keyspace, KeyspaceCreateOptions};
use miette::{miette, IntoDiagnostic, Result};

use crate::data::tuple::Tuple;
use crate::data::value::ValidityTs;
use crate::runtime::relation::decode_tuple_from_kv;
use crate::storage::{Storage, StoreTx};
use crate::utils::{swap_option_result, TempCollector};

/// Creates a Sled database object (backed by Fjall). Experimental.
/// You should use [`new_cozo_rocksdb`](crate::new_cozo_rocksdb) or
/// [`new_cozo_sqlite`](crate::new_cozo_sqlite) instead.
pub fn new_cozo_sled(path: impl AsRef<Path>) -> Result<crate::Db<SledStorage>> {
    let db = Database::builder(path.as_ref()).open().into_diagnostic()?;
    let partition = db
        .keyspace("default", KeyspaceCreateOptions::default)
        .into_diagnostic()?;
    let ret = crate::Db::new(SledStorage {
        db: Arc::new(db),
        partition,
    })?;

    ret.initialize()?;
    Ok(ret)
}

/// Storage engine using Sled (backed by Fjall)
#[derive(Clone)]
pub struct SledStorage {
    db: Arc<Database>,
    partition: Keyspace,
}

impl Storage<'_> for SledStorage {
    type Tx = SledTx;

    fn storage_kind(&self) -> &'static str {
        "sled"
    }

    fn transact(&self, _write: bool) -> Result<Self::Tx> {
        Ok(SledTx {
            partition: self.partition.clone(),
            db: self.db.clone(),
            changes: BTreeMap::new(),
        })
    }

    fn range_compact(&self, _lower: &[u8], _upper: &[u8]) -> Result<()> {
        Ok(())
    }

    fn batch_put<'a>(
        &'a self,
        data: Box<dyn Iterator<Item = Result<(Vec<u8>, Vec<u8>)>> + 'a>,
    ) -> Result<()> {
        let mut tx = self.transact(true)?;
        for result in data {
            let (key, val) = result?;
            tx.put(&key, &val)?;
        }
        tx.commit()?;
        Ok(())
    }
}

pub struct SledTx {
    partition: Keyspace,
    db: Arc<Database>,
    changes: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
}

impl<'s> StoreTx<'s> for SledTx {
    #[inline]
    fn get(&self, key: &[u8], _for_update: bool) -> Result<Option<Vec<u8>>> {
        if let Some(opt_val) = self.changes.get(key) {
            return Ok(opt_val.clone());
        }
        let ret = self.partition.get(key).into_diagnostic()?;
        Ok(ret.map(|v| v.to_vec()))
    }

    #[inline]
    fn put(&mut self, key: &[u8], val: &[u8]) -> Result<()> {
        self.changes.insert(key.to_vec(), Some(val.to_vec()));
        Ok(())
    }

    fn supports_par_put(&self) -> bool {
        false
    }

    #[inline]
    fn del(&mut self, key: &[u8]) -> Result<()> {
        self.changes.insert(key.to_vec(), None);
        Ok(())
    }

    fn del_range_from_persisted(&mut self, lower: &[u8], upper: &[u8]) -> Result<()> {
        let mut to_del = TempCollector::default();

        for pair in self.range_scan(lower, upper) {
            let (k, _) = pair?;
            to_del.push(k);
        }

        for k_res in to_del.into_iter() {
            self.partition.remove(&k_res).into_diagnostic()?;
        }
        Ok(())
    }

    #[inline]
    fn exists(&self, key: &[u8], _for_update: bool) -> Result<bool> {
        if let Some(opt_val) = self.changes.get(key) {
            return Ok(opt_val.is_some());
        }
        let ret = self.partition.get(key).into_diagnostic()?;
        Ok(ret.is_some())
    }

    fn commit(&mut self) -> Result<()> {
        if !self.changes.is_empty() {
            let mut batch = self.db.batch();
            for (k, opt_v) in &self.changes {
                if let Some(v) = opt_v {
                    batch.insert(&self.partition, k, v);
                } else {
                    batch.remove(&self.partition, k);
                }
            }
            batch.commit().into_diagnostic()?;
        }
        Ok(())
    }

    fn range_scan_tuple<'a>(
        &'a self,
        lower: &[u8],
        upper: &[u8],
    ) -> Box<dyn Iterator<Item = Result<Tuple>> + 'a>
    where
        's: 'a,
    {
        let changes_iter = self.changes.range(lower.to_vec()..upper.to_vec());
        let db_iter = self.partition.range(lower.to_vec()..upper.to_vec());
        Box::new(SledIter {
            changes_iter,
            db_iter,
            change_cache: None,
            db_cache: None,
        })
    }

    fn range_skip_scan_tuple<'a>(
        &'a self,
        _lower: &[u8],
        _upper: &[u8],
        _valid_at: ValidityTs,
    ) -> Box<dyn Iterator<Item = Result<Tuple>> + 'a> {
        Box::new(iter::once(Err(miette!(
            "Sled backend does not support time travelling."
        ))))
    }

    fn range_scan<'a>(
        &'a self,
        lower: &[u8],
        upper: &[u8],
    ) -> Box<dyn Iterator<Item = Result<(Vec<u8>, Vec<u8>)>> + 'a>
    where
        's: 'a,
    {
        let changes_iter = self.changes.range(lower.to_vec()..upper.to_vec());
        let db_iter = self.partition.range(lower.to_vec()..upper.to_vec());
        Box::new(SledIterRaw {
            changes_iter,
            db_iter,
            change_cache: None,
            db_cache: None,
        })
    }

    fn range_count<'a>(&'a self, lower: &[u8], upper: &[u8]) -> Result<usize>
    where
        's: 'a,
    {
        let changes_iter = self.changes.range(lower.to_vec()..upper.to_vec());
        let db_iter = self.partition.range(lower.to_vec()..upper.to_vec());
        Ok(SledIterRaw {
            changes_iter,
            db_iter,
            change_cache: None,
            db_cache: None,
        }
        .count())
    }

    fn total_scan<'a>(&'a self) -> Box<dyn Iterator<Item = Result<(Vec<u8>, Vec<u8>)>> + 'a>
    where
        's: 'a,
    {
        self.range_scan(&[], &[u8::MAX])
    }
}

struct SledIterRaw<'a, C, D>
where
    C: Iterator<Item = (&'a Vec<u8>, &'a Option<Vec<u8>>)>,
    D: Iterator<Item = fjall::Guard>,
{
    changes_iter: C,
    db_iter: D,
    change_cache: Option<(&'a Vec<u8>, &'a Option<Vec<u8>>)>,
    db_cache: Option<(Vec<u8>, Vec<u8>)>,
}

impl<'a, C, D> SledIterRaw<'a, C, D>
where
    C: Iterator<Item = (&'a Vec<u8>, &'a Option<Vec<u8>>)>,
    D: Iterator<Item = fjall::Guard>,
{
    #[inline]
    fn fill_cache(&mut self) -> Result<()> {
        if self.change_cache.is_none() {
            if let Some(res) = self.changes_iter.next() {
                self.change_cache = Some(res);
            }
        }

        if self.db_cache.is_none() {
            if let Some(res) = self.db_iter.next() {
                let (k, v) = res.into_inner().into_diagnostic()?;
                self.db_cache = Some((k.to_vec(), v.to_vec()));
            }
        }

        Ok(())
    }

    #[inline]
    fn next_inner(&mut self) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
        loop {
            self.fill_cache()?;
            match (&self.change_cache, &self.db_cache) {
                (None, None) => return Ok(None),
                (Some(_), None) => {
                    let (k, cv) = self.change_cache.take().unwrap();
                    if let Some(v) = cv {
                        return Ok(Some((k.clone(), v.clone())));
                    } else {
                        continue;
                    }
                }
                (None, Some(_)) => {
                    return Ok(self.db_cache.take());
                }
                (Some((ck, _)), Some((dk_key, _))) => match ck.as_slice().cmp(dk_key) {
                    Ordering::Less => {
                        let (k, cv) = self.change_cache.take().unwrap();
                        if let Some(v) = cv {
                            return Ok(Some((k.clone(), v.clone())));
                        } else {
                            continue;
                        }
                    }
                    Ordering::Greater => {
                        return Ok(self.db_cache.take());
                    }
                    Ordering::Equal => {
                        let (_, cv) = self.change_cache.take().unwrap();
                        let (dk_k, _) = self.db_cache.take().unwrap();
                        if let Some(v) = cv {
                            return Ok(Some((dk_k, v.clone())));
                        } else {
                            continue;
                        }
                    }
                },
            }
        }
    }
}

impl<'a, C, D> Iterator for SledIterRaw<'a, C, D>
where
    C: Iterator<Item = (&'a Vec<u8>, &'a Option<Vec<u8>>)>,
    D: Iterator<Item = fjall::Guard>,
{
    type Item = Result<(Vec<u8>, Vec<u8>)>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        swap_option_result(self.next_inner())
    }
}

struct SledIter<'a, C, D>
where
    C: Iterator<Item = (&'a Vec<u8>, &'a Option<Vec<u8>>)>,
    D: Iterator<Item = fjall::Guard>,
{
    changes_iter: C,
    db_iter: D,
    change_cache: Option<(&'a Vec<u8>, &'a Option<Vec<u8>>)>,
    db_cache: Option<(Vec<u8>, Vec<u8>)>,
}

impl<'a, C, D> SledIter<'a, C, D>
where
    C: Iterator<Item = (&'a Vec<u8>, &'a Option<Vec<u8>>)>,
    D: Iterator<Item = fjall::Guard>,
{
    #[inline]
    fn fill_cache(&mut self) -> Result<()> {
        if self.change_cache.is_none() {
            if let Some(res) = self.changes_iter.next() {
                self.change_cache = Some(res);
            }
        }

        if self.db_cache.is_none() {
            if let Some(res) = self.db_iter.next() {
                let (k, v) = res.into_inner().into_diagnostic()?;
                self.db_cache = Some((k.to_vec(), v.to_vec()));
            }
        }

        Ok(())
    }

    #[inline]
    fn next_inner(&mut self) -> Result<Option<Tuple>> {
        loop {
            self.fill_cache()?;
            match (&self.change_cache, &self.db_cache) {
                (None, None) => return Ok(None),
                (Some(_), None) => {
                    let (k, cv) = self.change_cache.take().unwrap();
                    if let Some(v) = cv {
                        return Ok(Some(decode_tuple_from_kv(k, v, None)?));
                    } else {
                        continue;
                    }
                }
                (None, Some(_)) => {
                    let (k, v) = self.db_cache.take().unwrap();
                    return Ok(Some(decode_tuple_from_kv(&k, &v, None)?));
                }
                (Some((ck, _)), Some((dk_key, _))) => match ck.as_slice().cmp(dk_key) {
                    Ordering::Less => {
                        let (k, cv) = self.change_cache.take().unwrap();
                        if let Some(v) = cv {
                            return Ok(Some(decode_tuple_from_kv(k, v, None)?));
                        } else {
                            continue;
                        }
                    }
                    Ordering::Greater => {
                        let (k, v) = self.db_cache.take().unwrap();
                        return Ok(Some(decode_tuple_from_kv(&k, &v, None)?));
                    }
                    Ordering::Equal => {
                        let (_, cv) = self.change_cache.take().unwrap();
                        let (dk_k, _) = self.db_cache.take().unwrap();
                        if let Some(v) = cv {
                            return Ok(Some(decode_tuple_from_kv(&dk_k, v, None)?));
                        } else {
                            continue;
                        }
                    }
                },
            }
        }
    }
}

impl<'a, C, D> Iterator for SledIter<'a, C, D>
where
    C: Iterator<Item = (&'a Vec<u8>, &'a Option<Vec<u8>>)>,
    D: Iterator<Item = fjall::Guard>,
{
    type Item = Result<Tuple>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        swap_option_result(self.next_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn fjall_put_get_round_trip() -> Result<()> {
        let dir = tempdir().into_diagnostic()?;
        let db = new_cozo_sled(dir.path())?;

        let tx = db.db.transact(true)?;
        assert!(tx.get(b"non-existent", false)?.is_none());

        let mut tx = db.db.transact(true)?;
        tx.put(b"k1", b"v1")?;
        assert_eq!(tx.get(b"k1", false)?.unwrap(), b"v1");

        // Before commit, not visible globally
        let tx2 = db.db.transact(false)?;
        assert!(tx2.get(b"k1", false)?.is_none());

        tx.commit()?;

        let tx3 = db.db.transact(false)?;
        assert_eq!(tx3.get(b"k1", false)?.unwrap(), b"v1");

        Ok(())
    }

    #[test]
    fn fjall_delete_hides_committed_key() -> Result<()> {
        let dir = tempdir().into_diagnostic()?;
        let db = new_cozo_sled(dir.path())?;

        let mut tx = db.db.transact(true)?;
        tx.put(b"k1", b"v1")?;
        tx.commit()?;

        let mut tx2 = db.db.transact(true)?;
        tx2.del(b"k1")?;
        assert!(tx2.get(b"k1", false)?.is_none());
        tx2.commit()?;

        let tx3 = db.db.transact(false)?;
        assert!(tx3.get(b"k1", false)?.is_none());

        Ok(())
    }

    #[test]
    fn fjall_range_scan_merges_uncommitted_changes() -> Result<()> {
        let dir = tempdir().into_diagnostic()?;
        let db = new_cozo_sled(dir.path())?;

        let mut tx = db.db.transact(true)?;
        tx.put(b"k2", b"v2")?;
        tx.put(b"k4", b"v4")?;
        tx.commit()?;

        let mut tx2 = db.db.transact(true)?;
        tx2.put(b"k1", b"v1")?; // uncommitted insert
        tx2.del(b"k2")?; // uncommitted delete
        tx2.put(b"k3", b"v3")?; // uncommitted insert
                                // k4 remains unchanged

        let items: Vec<_> = tx2.range_scan(b"k0", b"k9").collect::<Result<Vec<_>>>()?;
        assert_eq!(
            items,
            vec![
                (b"k1".to_vec(), b"v1".to_vec()),
                (b"k3".to_vec(), b"v3".to_vec()),
                (b"k4".to_vec(), b"v4".to_vec()),
            ]
        );

        let count = tx2.range_count(b"k0", b"k9")?;
        assert_eq!(count, 3);

        Ok(())
    }
}
