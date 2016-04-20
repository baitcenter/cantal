==================
Carbon Integration
==================

Carbon_ integration allows to use cantal as an agent for carbon, so you
can view the data in graphite_ or any other carbon-compatible system (such
as a graphana_)

Basically this allows you to view recent data in cantal and use carbon for
archival of statistics

.. note:: The support is currently far from be comprehensive. Only some data
   can be sent to carbon. Sending whole collected statistics to graphite is
   too much, so we adding features one by one.

.. _carbon: http://graphite.wikidot.com/
.. _graphite: http://graphite.wikidot.com/
.. _graphana: http://grafana.org/



Configuration
=============

Cantal starting with v0.3.0, has a default configuration directory
``/etc/cantal``. You need to put some configuration file there:

.. code-block:: yaml

   # /etc/cantal/localhost.carbon.yaml
   host: 127.0.0.1  # only IP address supported in v0.3.0
   port: 2003
   interval: 10
   enable-cgroup-stats: true

All configurations which end with ``.carbon.yaml`` will be read. Multiple
configurations may be used, each configuration is a separate connection with
it's own set of metrics.

Options:

host
    (required) The **IP address** to send data to. *Hostnames are not
    supported yet*.

port
    (default ``2003``) Port where carbon_ listens with text protocol.
    The default matches the same of carbon.

interval
    (default ``10``) Interval of sending data to carbon. The cantal's
    collection interval is ``2`` seconds for most metrics. But there is no
    much value of sending such detailed statistics to carbon. Cantal will
    provide 1 hour of highest precision history in it's own interface and send
    averages of the values to a carbon.

enable-cgroup-stats
    (default ``false``) Send data about cgroups to carbon


Metrics Layout
==============

By default cantal sends nothing, even if connection params are set.

CGroup statistics (enabled with ``enable-cgroup-stats``):

* ``cantal.<HOSTNAME>.cgroups.<GROUP_NAME>.<METRIC_NAME>``

    * Metrics (all represent the sum for all processes in the group):

        * ``vsize`` -- virtual memory size
        * ``rss`` -- resident set size
        * ``num_processes`` -- total number of processes in the group
        * ``num_threads`` -- total number of threads in the group
        * ``user_cpu_percent`` -- percentage of CPU spent in user mode
        * ``system_cpu_percent`` -- percentage of CPU spent in system mode
        * ``read_bps`` -- average bytes per second read on disk
        * ``writes_bps`` -- average bytes per second written to disk

    * Ggroup is a dot-delimited hierarchy of cgroups with systemd-like
      suffixes removed, for example:
      ``/sys/fs/cgroup/systemd/system.slice/nscd.service`` will turn
      into ``system.nscd``
    * The ``.swap`` and ``.mount`` (systemd-specific) groups are skipped
    * The root group ``user`` (upstart- and systemd-specific) group is ignored
    * If the process is in group ``a.b`` it will not count for group ``a``,
      the statistics for ``a`` contains only processes immediately in the group

* ``cantal.<HOSTNAME>.cgroups.<GROUP_NAME>.states.<STATE_NAME>.<METRIC_NAME>``
  -- application-submitted metrics which have a ``state`` value
* ``cantal.<HOSTNAME>.cgroups.<GROUP_NAME>.groups.<STATE_NAME>.<METRIC_NAME>``
  -- application-submitted metrics which have a ``group`` value


