import assert from 'node:assert/strict';
import { embeddedArchives } from './prepare-windows-vulkan.mjs';
import { nsisBinary } from './verify-windows-installer.mjs';

// A small complete archive from the pinned SDK: malformed embedded headers must never be extracted.
const archive = Buffer.from('N3q8ryccAANYGmF6AAAAAAAAAABDAAAAAAAAAOMD20oBBQEOAYARDQBjAG0AYQBrAGUAAAAUCgEAXUVLmwNr3QESCgEApSHD7/dq3QETCgEAuyeenANr3QEVBgEAEID/QQAA', 'base64');
const wrapped = Buffer.concat([Buffer.from('executable prefix'), archive, Buffer.from('trailing data')]);
assert.deepEqual(embeddedArchives(wrapped), [archive]);
assert.deepEqual(embeddedArchives(Buffer.alloc(0)), []);
assert.deepEqual(embeddedArchives(archive.subarray(0, 24)), []);
assert.deepEqual(embeddedArchives(archive.subarray(0, archive.length - 1)), []);
const corrupt = Buffer.from(archive);
corrupt[8] ^= 1;
assert.deepEqual(embeddedArchives(corrupt), []);
assert.deepEqual(embeddedArchives(Buffer.concat([corrupt, archive])), [archive]);
const binary = Buffer.from('prefix__TAURI_BUNDLE_TYPE_VAR_UNKsuffix');
assert.equal(nsisBinary(binary).toString(), 'prefix__TAURI_BUNDLE_TYPE_VAR_NSSsuffix');
assert.equal(binary.toString(), 'prefix__TAURI_BUNDLE_TYPE_VAR_UNKsuffix');
assert.deepEqual(nsisBinary(Buffer.from('ordinary bytes')), Buffer.from('ordinary bytes'));
console.log('Windows packaging: embedded archive bounds, truncation and checksum rejection passed.');
