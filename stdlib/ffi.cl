//! Conlang FFI
use super::preamble::*;

type void = ();

#[extern("C")]
/// void free(void *_Nullable ptr);
/// 
/// Frees a block of memory previously allocated with `malloc`
fn free(ptr: *void);

#[extern("C")]
/// void *malloc(size_t size);
/// 
/// Allocates a block of uninitialized memory
fn malloc(size: usize) -> *void;

#[extern("C")]
/// void *calloc(size_t n, size_t size);
/// 
/// Allocates a block of zero-initialized memory
fn calloc(n: usize, size: usize);

#[extern("C")]
/// void *realloc(void *_Nullable ptr, size_t size);
/// 
/// Reallocates a block of memory to fit `size` bytes
fn realloc(ptr: *void, size: usize) -> *void;

#[extern("C")]
/// void *reallocarray(void *_Nullable ptr, size_t n, size_t size);
/// 
/// Reallocates a block of memory to fit `n` elements of `size` bytes.
fn reallocarray(ptr: *void, n: usize, size: usize) -> *void;
