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

# Coordinates for app and Applications folder in the window
icon_locations = {
    appname: (180, 120),       # App icon
    "Applications": (460, 120) # Applications folder symlink
}

# -------------------------------------------------------------------
# Volume & window
# -------------------------------------------------------------------

volume_name = defines.get("volume_name", appname.replace(".app", ""))
format = "UDBZ"

window_rect = ((200, 120), (640, 360))
default_view = "icon-view"

show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False

# -------------------------------------------------------------------
# Background
# -------------------------------------------------------------------

background = "builtin-arrow"

# -------------------------------------------------------------------
# Icon view configuration
# -------------------------------------------------------------------

arrange_by = None
grid_spacing = 100
label_pos = "bottom"
text_size = 16
icon_size = 100
