class Unchecked(Exception):
    """A legal claim this replay cannot complete (§12.2 host limit / scope)."""
    def __init__(self, message, section="12.1"):
        super().__init__(message)
        self.message = message
        self.section = section


class Reject(Exception):
    """A claim obligation failed; carries a section-cited diagnostic."""
    def __init__(self, code, section, path, message, witness=None):
        super().__init__(message)
        self.diag = {"code": code, "section": section, "path": path, "message": message}
        if witness is not None:
            self.diag["witness"] = witness



class ObjectError(Exception):
    """The object itself violates a stated well-formedness rule the replay relies on."""
    def __init__(self, section, message):
        super().__init__(message)
        self.section = section
        self.message = message
