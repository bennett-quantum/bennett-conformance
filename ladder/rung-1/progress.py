"""Publish complete checkpoints for a single worker's progress file."""
import json


def write_summary(path, summary):
    pending = path.with_name(path.name+'.pending')
    pending.write_text(json.dumps(summary)+'\n')
    pending.replace(path)
