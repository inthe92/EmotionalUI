#!/usr/bin/env python3
"""Lossless preparation of shared EmulationStation emulator preferences.

This module only transforms bytes in memory. It never writes settings or
launches software. A future caller must provide a separately reviewed,
supervised, atomic persistence transaction and validate installed choices.
"""
import html
import re
import xml.parsers.expat as expat

LIMIT = 1024 * 1024
WRAPPER = b'<emotional-settings-fragment>'
END_WRAPPER = b'</emotional-settings-fragment>'


def prepare_emulator_preferences(raw, system, emulator, core):
    """Return edited bytes; preserve all unrelated bytes and fail on ambiguity."""
    if (not isinstance(raw, bytes) or len(raw) > LIMIT or b'<!DOCTYPE' in raw.upper()
            or b'<!ENTITY' in raw.upper()):
        raise ValueError('Invalid or oversized settings')
    if (not isinstance(system, str) or not system or len(system) > 96
            or any(ord(c) < 32 for c in system)):
        raise ValueError('Invalid exact system name')
    if any(not re.fullmatch(r'[A-Za-z0-9_+-]{1,96}', value or '') for value in (emulator, core)):
        raise ValueError('Invalid emulator or core identifier')
    raw.decode('utf-8-sig')
    prefix = re.match(rb'^(?:\xef\xbb\xbf)?\s*(?:<\?xml[^>]*\?>)?', raw).end()
    body = raw[prefix:]
    wrapped = WRAPPER + body + END_WRAPPER
    parser = expat.ParserCreate()
    stack, top, strings, closes = [], [], [], {}

    def start(tag, attrs):
        stack.append(tag)
        if len(stack) == 2:
            top.append(tag)
        if tag == 'string' and 'name' in attrs:
            strings.append((tuple(stack), attrs, parser.CurrentByteIndex - len(WRAPPER)))

    def end(tag):
        if len(stack) == 2 and tag == 'config':
            closes['config'] = parser.CurrentByteIndex - len(WRAPPER)
        stack.pop()

    parser.StartElementHandler = start
    parser.EndElementHandler = end
    try:
        parser.Parse(wrapped, True)
    except expat.ExpatError as error:
        raise ValueError('Unreadable settings') from error
    container = ('emotional-settings-fragment', 'config') if top == ['config'] else ('emotional-settings-fragment',)
    if 'config' in top and top != ['config']:
        raise ValueError('Ambiguous settings container')
    updates = {system+'.emulator': emulator, system+'.core': core}
    replacements = []
    for key, value in updates.items():
        found = [(path, attrs, offset) for path, attrs, offset in strings if attrs['name'] == key]
        if len(found) > 1 or (found and found[0][0][:-1] != container):
            raise ValueError('Ambiguous preference')
        encoded = html.escape(value, quote=True).encode('utf-8')
        if not found:
            continue
        _, attrs, offset = found[0]
        if 'value' not in attrs or offset < 0:
            raise ValueError('Preference has no value')
        # Scan only this XML opening tag, respecting quoted > characters.
        end_offset, quote = offset, None
        while end_offset < len(body):
            byte = body[end_offset]
            if quote is None and byte in (34, 39):
                quote = byte
            elif byte == quote:
                quote = None
            elif quote is None and byte == 62:
                break
            end_offset += 1
        if end_offset >= len(body):
            raise ValueError('Unterminated preference')
        tag = body[offset:end_offset+1]
        attributes = re.finditer(rb'\s+([^\s=/>]+)\s*=\s*(["\'])(.*?)\2', tag, re.S)
        match = next((m for m in attributes if m.group(1) == b'value'), None)
        if not match:
            raise ValueError('Cannot safely locate preference value')
        replacements.append((offset+match.start(3), offset+match.end(3), encoded))
    missing = [key for key in updates if not any(attrs['name'] == key for _, attrs, _ in strings)]
    if missing:
        offset = closes.get('config', -1) if container[-1] == 'config' else len(body)
        if offset < 0 or (container[-1] == 'config' and not body[offset:].startswith(b'</config')):
            raise ValueError('Cannot safely insert settings')
        addition = ''.join('\n<string name="'+html.escape(key, quote=True)+'" value="'
                           +html.escape(updates[key], quote=True)+'"/>' for key in missing).encode('utf-8')
        replacements.append((offset, offset, addition+b'\n'))
    for start_at, end_at, value in sorted(replacements, reverse=True):
        body = body[:start_at] + value + body[end_at:]
    result = raw[:prefix] + body
    if len(result) > LIMIT:
        raise ValueError('Edited settings exceed size limit')
    return result
