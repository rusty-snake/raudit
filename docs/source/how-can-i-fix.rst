How can I fix
=============

.. note::

  raudit is a tool that provides an impression of the security of profiles.
  It does not tell the truth, instead it uses heuristics and simplifications.
  **Do not optimize your profile against a metric**, optimize it against the
  real world. Some programs may not work without the permissions reported by
  raudit and others aren't useful anymore.

``BAD: no_new_privs is NOT set, the sandbox can acquire new privileges using execve.``
--------------------------------------------------------------------------------------

Add ``nonewprivs`` to the profile.

``BAD: The capability bounding set is NOT empty.``
--------------------------------------------------

Add ``caps.drop all`` to the profile.

``UGLY: The sandbox can write to $HOME/... after a chmod.``
-----------------------------------------------------------

Add ``read-only`` or ``blacklist`` for this path or refactor the profile
as a whitelisting profile.

``UGLY: The sandbox can create $HOME/...``
------------------------------------------

Add ``read-only`` or ``blacklist`` for an existing parent path or refactor the
profile as a whitelisting profile.

``UGLY: The sandbox can write to $HOME/...``
--------------------------------------------

Add ``read-only`` or ``blacklist`` for this path or refactor the profile
as a whitelisting profile.

``UGLY: The sandbox can read $HOME/...``
-----------------------------------------

You have three options (from easy and weak to difficult and tight):

 1. Add a ``blacklist`` for this path.
 2. If this path is in one of the disable-\*.inc includes,
    add the corresponding disable-\*.inc include.
 3. Refactor the profile as a whitelisting profile.
