######
raudit
######

a configurable audit program for firejail-sandboxes with metrics
################################################################

:Version: 0.1.0
:Manual section: 7

SYNOPSIS
========

.. code-block:: bash

  firejail [OPTIONS] --profile=[PROFILE-NAME] /path/to/raudit < /path/to/RULES

DESCRIPTION
===========

raudit is a audit program for firejail sandboxes. Unlike faudit, the audit
program distributed with firejail in the past, raudit reads a list of rules from
stdin instead of checking hardcoded rules.

ENVIRONMENT
===========

$RAUDIT_ARGS
------------

If set, command-line arguments are read from it instead of the command-line.

EXAMPLES
========

.. code-block:: bash

  firejail --profile=inkscape /proc/self/fd/3 </usr/local/share/raudit/default.rules 3</usr/local/libexec/raudit

SEE ALSO
========

firejail(1)
