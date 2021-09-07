bl_info = {
    "name": "pbrt-v3 exporter",
    "blender": (2, 92, 0),
    "category": "Import-Export",
}

from . import export

# Reloading the script does not reload imports without this
if "bpy" in locals():
    import importlib

    importlib.reload(export)

import bpy


def register():
    for cls in export.auto_register(True):
        bpy.utils.register_class(cls)


def unregister():
    for cls in export.auto_register(False):
        bpy.utils.unregister_class(cls)
