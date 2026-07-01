"""测试 EventLogger 和事件解析逻辑 — 无需 DLL。"""

from __future__ import annotations

import io
import sys

import pytest
from scripts.tld_interface import EventLogger


def test_start_event_output():
    logger = EventLogger()
    event = {"Type": "start", "ShowName": "", "ID": ""}
    msg = {}
    buf = io.StringIO()
    stdout = sys.stdout
    sys.stdout = buf
    try:
        logger(event, msg)
    finally:
        sys.stdout = stdout
    output = buf.getvalue()
    assert "下载会话开始" in output


def test_start_one_event_output():
    logger = EventLogger()
    event = {"Type": "startOne", "ShowName": "test.zip", "ID": "1"}
    msg = {"URL": "https://example.com/test.zip", "Index": 1, "Total": 3}
    buf = io.StringIO()
    stdout = sys.stdout
    sys.stdout = buf
    try:
        logger(event, msg)
    finally:
        sys.stdout = stdout
    output = buf.getvalue()
    assert "test.zip" in output
    assert "1/3" in output


def test_update_event_output():
    logger = EventLogger()
    event = {"Type": "update", "ShowName": "test.zip", "ID": "1"}
    msg = {"Total": 1000, "Downloaded": 500}
    buf = io.StringIO()
    stdout = sys.stdout
    sys.stdout = buf
    try:
        logger(event, msg)
    finally:
        sys.stdout = stdout
    output = buf.getvalue()
    assert "50.00%" in output or "50" in output


def test_end_one_event_output():
    logger = EventLogger()
    event = {"Type": "endOne", "ShowName": "test.zip", "ID": "1"}
    msg = {"URL": "https://example.com/test.zip", "Index": 1, "Total": 3}
    buf = io.StringIO()
    stdout = sys.stdout
    sys.stdout = buf
    try:
        logger(event, msg)
    finally:
        sys.stdout = stdout
    output = buf.getvalue()
    assert "test.zip" in output


def test_end_event_output():
    logger = EventLogger()
    event = {"Type": "end"}
    msg = {}
    buf = io.StringIO()
    stdout = sys.stdout
    sys.stdout = buf
    try:
        logger(event, msg)
    finally:
        sys.stdout = stdout
    output = buf.getvalue()
    assert "全部下载完成" in output


def test_err_event_output():
    logger = EventLogger()
    event = {"Type": "err", "ShowName": "test.zip", "ID": "1"}
    msg = {"Error": "Connection timeout"}
    buf = io.StringIO()
    stdout = sys.stdout
    sys.stdout = buf
    try:
        logger(event, msg)
    finally:
        sys.stdout = stdout
    output = buf.getvalue()
    assert "Connection timeout" in output


def test_msg_event_output():
    logger = EventLogger()
    event = {"Type": "msg", "ShowName": "test.zip", "ID": "1"}
    msg = {"Text": "正在重试"}
    buf = io.StringIO()
    stdout = sys.stdout
    sys.stdout = buf
    try:
        logger(event, msg)
    finally:
        sys.stdout = stdout
    output = buf.getvalue()
    assert "正在重试" in output


def test_unknown_event_type():
    logger = EventLogger()
    event = {"Type": "unknown_event_type_xyz"}
    msg = {}
    buf = io.StringIO()
    stdout = sys.stdout
    sys.stdout = buf
    try:
        logger(event, msg)
    finally:
        sys.stdout = stdout
    output = buf.getvalue()
    assert "未知事件" in output


def test_eventlogger_is_callable():
    logger = EventLogger()
    assert callable(logger)


def test_empty_event():
    logger = EventLogger()
    event = {}
    msg = {}
    buf = io.StringIO()
    stdout = sys.stdout
    sys.stdout = buf
    try:
        logger(event, msg)
    finally:
        sys.stdout = stdout
    output = buf.getvalue()
    assert "未知事件" in output or output == ""
