from __future__ import annotations

import io
import json
import unittest

from scriptotar_sidecar.errors import SidecarError
from scriptotar_sidecar.protocol import ProtocolWriter, parse_command_line
from scriptotar_sidecar.version import PROTOCOL_VERSION


class ProtocolTests(unittest.TestCase):
    def test_ping_parses(self):
        command = parse_command_line(json.dumps({"protocol": PROTOCOL_VERSION, "type": "ping", "request_id": "r1"}))
        self.assertEqual(command.type, "ping")
        self.assertEqual(command.payload["request_id"], "r1")

    def test_malformed_command(self):
        with self.assertRaises(SidecarError) as raised:
            parse_command_line("{broken")
        self.assertEqual(raised.exception.code, "MALFORMED_JSON")

    def test_unsupported_protocol_version(self):
        with self.assertRaises(SidecarError) as raised:
            parse_command_line(json.dumps({"protocol": 99, "type": "ping"}))
        self.assertEqual(raised.exception.code, "UNSUPPORTED_PROTOCOL")

    def test_unknown_command(self):
        with self.assertRaises(SidecarError) as raised:
            parse_command_line(json.dumps({"protocol": PROTOCOL_VERSION, "type": "explode"}))
        self.assertEqual(raised.exception.code, "UNKNOWN_COMMAND")

    def test_writer_emits_one_json_object_per_line(self):
        stream = io.StringIO()
        ProtocolWriter(stream).emit("pong", request_id="abc")
        lines = stream.getvalue().splitlines()
        self.assertEqual(len(lines), 1)
        event = json.loads(lines[0])
        self.assertEqual(event["protocol"], PROTOCOL_VERSION)
        self.assertEqual(event["type"], "pong")
