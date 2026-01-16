import os
import plistlib

# -------------------------------------------------------------------
# Application
# -------------------------------------------------------------------

application = defines.get("app")  # passed via -D app=/path/to/App.app
appname = os.path.basename(application)

# -------------------------------------------------------------------
# Files & layout
# -------------------------------------------------------------------

files = [application]
symlinks = {"Applications": "/Applications"}

icon_locations = {appname: (140, 120), "Applications": (500, 120)}

# -------------------------------------------------------------------
# Volume & window
# -------------------------------------------------------------------

volume_name = defines.get("volume_name", appname.replace(".app", ""))
format = "UDBZ"

window_rect = ((100, 100), (640, 280))
default_view = "icon-view"

show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False

# -------------------------------------------------------------------
# Background
# -------------------------------------------------------------------

# background = "platform/macos/installer_background.png"
background = "builtin-arrow"

# -------------------------------------------------------------------
# Icon view configuration
# -------------------------------------------------------------------

arrange_by = None
grid_spacing = 100
label_pos = "bottom"
text_size = 16
icon_size = 128
