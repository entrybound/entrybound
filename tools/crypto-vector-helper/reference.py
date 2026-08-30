#!/usr/bin/env python3
"""Independent, non-production crypto-wire vector generator.

All inputs are fixed public test data. This is not an encryption API.
"""

from __future__ import annotations

import argparse
import hashlib
import hmac
import struct


FORMAT_NAMESPACE = b"ecf/bootstrap-v1"
XWING_PARAMETERS = b"xwing-mlkem768-x25519-sha3-256/draft-10"


def t1(label: str, fields: list[bytes]) -> bytes:
    encoded = struct.pack(">H", len(label.encode("ascii"))) + label.encode("ascii")
    encoded += struct.pack(">H", len(fields))
    for tag, value in enumerate(fields, 1):
        encoded += struct.pack(">HQ", tag, len(value)) + value
    return encoded


def hkdf_extract(salt: bytes, ikm: bytes) -> bytes:
    return hmac.new(salt, ikm, hashlib.sha256).digest()


def hkdf_expand(prk: bytes, info: bytes, length: int = 32) -> bytes:
    output = b""
    previous = b""
    counter = 1
    while len(output) < length:
        previous = hmac.new(
            prk, previous + info + bytes([counter]), hashlib.sha256
        ).digest()
        output += previous
        counter += 1
    return output[:length]


def method_context(
    stanza_type: int,
    protection_class: int,
    stanza_id: bytes,
    parameters: bytes,
    encapsulation: bytes,
) -> bytes:
    return t1(
        "entrybound/recipient-method-context/v1",
        [
            struct.pack(">H", 1),
            struct.pack(">H", stanza_type),
            bytes([protection_class]),
            stanza_id,
            bytes(16),
            parameters,
            encapsulation,
        ],
    )


def wrap_key(
    archive_id: bytes,
    method_secret: bytes,
    stanza_type: int,
    protection_class: int,
    stanza_id: bytes,
    context_digest: bytes,
) -> tuple[bytes, bytes]:
    prk = hkdf_extract(archive_id, method_secret)
    info = t1(
        "entrybound/recipient-wrap-key/v1",
        [
            struct.pack(">H", 1),
            struct.pack(">H", stanza_type),
            bytes([protection_class]),
            stanza_id,
            context_digest,
        ],
    )
    return prk, hkdf_expand(prk, info)


def wrap_ad(
    archive_id: bytes,
    stanza_type: int,
    protection_class: int,
    stanza_id: bytes,
    parameters: bytes,
    encapsulation: bytes,
    nonce: bytes,
) -> bytes:
    return t1(
        "entrybound/recipient-wrap-ad/v1",
        [
            FORMAT_NAMESPACE,
            struct.pack(">H", 0),
            struct.pack(">H", 1),
            struct.pack(">H", 1),
            struct.pack(">H", 1),
            archive_id,
            struct.pack(">H", 1),
            struct.pack(">H", stanza_type),
            bytes([protection_class]),
            stanza_id,
            bytes(16),
            parameters,
            encapsulation,
            nonce,
        ],
    )


def a2id_parameters(salt: bytes) -> bytes:
    return b"A2ID" + struct.pack(">IIII", 19, 262_144, 3, 4) + salt


def field(tag: int, field_type: int, value: bytes) -> bytes:
    return struct.pack(">HBBQ", tag, field_type, 0, len(value)) + value


def recipient_directory_entry(
    stanza_id: bytes, fingerprint: bytes, label: str
) -> bytes:
    payload = b"".join(
        [
            field(1, 7, stanza_id),
            field(2, 2, struct.pack(">H", 1)),
            field(3, 7, fingerprint),
            field(4, 8, label.encode("utf-8")),
        ]
    )
    return struct.pack(">HHIQ", 22, 1, 0, len(payload)) + payload


def sequence(kind: int, items: list[bytes]) -> bytes:
    encoded = b"EBCS" + struct.pack(">HHIQ", 1, kind, 0, len(items))
    for item in items:
        encoded += struct.pack(">Q", len(item)) + item
    return encoded


def private_object(kind: int, payload: bytes) -> bytes:
    return b"EBPO" + struct.pack(">HHI", 1, kind, 0) + payload


def emit(name: str, value: bytes) -> None:
    print(f"{name}={value.hex()}")


def emit_sha(name: str, value: bytes) -> None:
    emit(name, hashlib.sha256(value).digest())


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--canonical-only", action="store_true")
    args = parser.parse_args()

    archive_id = bytes(range(0x00, 0x20))
    afk = bytes(range(0x20, 0x40))

    v5_secret = bytes(range(0xB0, 0xD0))
    v5_id = bytes(range(0x40, 0x50))
    v5_nonce = bytes(range(0x50, 0x5C))
    v5_encapsulation = bytes(i % 256 for i in range(1120))
    v5_context = method_context(1, 1, v5_id, XWING_PARAMETERS, v5_encapsulation)
    v5_digest = hashlib.sha256(v5_context).digest()
    v5_prk, v5_key = wrap_key(archive_id, v5_secret, 1, 1, v5_id, v5_digest)
    v5_ad = wrap_ad(
        archive_id, 1, 1, v5_id, XWING_PARAMETERS, v5_encapsulation, v5_nonce
    )
    emit("V5_METHOD_CONTEXT", v5_context)
    emit("V5_METHOD_CONTEXT_SHA256", v5_digest)
    emit_sha("V5_ENCAPSULATION_SHA256", v5_encapsulation)
    emit("V5_WRAP_PRK", v5_prk)
    emit("V5_WRAP_KEY", v5_key)
    emit("V5_WRAP_NONCE", v5_nonce)
    emit("V5_AFK", afk)
    emit("V5_WRAP_AD", v5_ad)
    emit_sha("V5_WRAP_AD_SHA256", v5_ad)

    v6_salt = bytes(range(0x60, 0x70))
    v6_parameters = a2id_parameters(v6_salt)
    v6_id = bytes(range(0x70, 0x80))
    v6_nonce = bytes(range(0x80, 0x8C))
    if args.canonical_only:
        # The frozen Argon2 output is independently verified in full mode.
        v6_secret = bytes.fromhex(
            "b954ca2999c51dfbd1810dad53340641"
            "d507696a416ec59334f7e18bf823ea2d"
        )
    else:
        from argon2.low_level import Type, hash_secret_raw

        v6_secret = hash_secret_raw(
            b"correct horse battery staple",
            v6_salt,
            time_cost=3,
            memory_cost=262_144,
            parallelism=4,
            hash_len=32,
            type=Type.ID,
            version=19,
        )
    v6_context = method_context(2, 2, v6_id, v6_parameters, b"")
    v6_digest = hashlib.sha256(v6_context).digest()
    v6_prk, v6_key = wrap_key(archive_id, v6_secret, 2, 2, v6_id, v6_digest)
    v6_ad = wrap_ad(archive_id, 2, 2, v6_id, v6_parameters, b"", v6_nonce)
    emit("V6_A2ID", v6_parameters)
    emit("V6_ARGON2ID_OUTPUT", v6_secret)
    emit("V6_METHOD_CONTEXT", v6_context)
    emit("V6_METHOD_CONTEXT_SHA256", v6_digest)
    emit("V6_WRAP_PRK", v6_prk)
    emit("V6_WRAP_KEY", v6_key)
    emit("V6_WRAP_NONCE", v6_nonce)
    emit("V6_AFK", afk)
    emit("V6_WRAP_AD", v6_ad)
    emit_sha("V6_WRAP_AD_SHA256", v6_ad)

    if not args.canonical_only:
        from cryptography.hazmat.primitives.ciphers.aead import AESGCMSIV

        emit("V5_WRAPPED_AFK", AESGCMSIV(v5_key).encrypt(v5_nonce, afk, v5_ad))
        emit("V6_WRAPPED_AFK", AESGCMSIV(v6_key).encrypt(v6_nonce, afk, v6_ad))

    entry_a = recipient_directory_entry(bytes(range(16)), bytes(range(0x20, 0x40)), "alice")
    entry_b = recipient_directory_entry(bytes(range(0x10, 0x20)), bytes(range(0x40, 0x60)), "")
    empty = sequence(1, [])
    one = sequence(8, [entry_a])
    multi = sequence(8, [entry_a, entry_b])
    private = private_object(3, multi)
    emit("S1_EMPTY", empty)
    emit_sha("S1_EMPTY_SHA256", empty)
    emit("S2_ONE", one)
    emit_sha("S2_ONE_SHA256", one)
    emit("S3_MULTI", multi)
    emit_sha("S3_MULTI_SHA256", multi)
    emit("S4_PRIVATE_CONTROL_OBJECT", private)
    emit_sha("S4_PRIVATE_CONTROL_OBJECT_SHA256", private)
    reversed_items = sequence(8, [entry_b, entry_a])
    duplicate = sequence(8, [entry_a, entry_a])
    truncated = one[:-1]
    emit("S5_OUT_OF_ORDER_INVALID", reversed_items)
    emit_sha("S5_OUT_OF_ORDER_INVALID_SHA256", reversed_items)
    emit("S6_DUPLICATE_INVALID", duplicate)
    emit_sha("S6_DUPLICATE_INVALID_SHA256", duplicate)
    emit("S7_TRUNCATED_INVALID", truncated)
    emit_sha("S7_TRUNCATED_INVALID_SHA256", truncated)


if __name__ == "__main__":
    main()
