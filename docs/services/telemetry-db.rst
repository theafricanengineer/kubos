Telemetry Database Service
==========================

KubOS utilizes a `SQLite database <https://www.sqlite.org/about.html>`__ to store telemetry data generated by the
hardware and payload services until it is requested for transmission by the ground station.

SQLite only allows one process to make changes to a database at a time, so the telemetry database service acts as a 
single point of contact for interacting with the underlying telemetry database.

Interface Details
-----------------

Specific details about the available GraphQL queries can be found in the |telem-db| Rust docs.

 .. |telem-db| raw:: html
 
    <a href="../rust-docs/telemetry_service/index.html" target="_blank">telemetry database service</a>
    
Benchmark
~~~~~~~~~

The Kubos repo contains a `database benchmark project <https://github.com/kubos/kubos/tree/master/test/benchmark/db-test>`__
which we have used to measure various behaviors of the telemetry database service.

Because each OBC has its own unique system resources, we recommend :doc:`compiling and running <../sdk-docs/sdk-rust>`
the test project on your OBC to obtain the most accurate results.

When run on a Beaglebone Black, we gathered the following benchmark statistics:

- Sending UDP requests takes ~45 microseconds

    - This means that a client can send UDP requests at a rate of 22,000 requests per second, if they don't wait for
      a response. Note: This is far faster than rate at which the service processes requests, meaning that packets
      will be dropped if this maximum speed is used.
    
- Telemetry database inserts take ~16 milliseconds
- Round-trip service transactions (including UDP receive request, database insert, and UDP send response) take ~17 milliseconds

    - This means that the service can process roughly 58 database insert requests per second

Querying the Service
--------------------

The ``telemetry`` query can be used to fetch a certain selection of data from the telemetry database.
It will return an array of database entries.

The query has the following schema::

    query {
        telemetry(timestampGe: Integer, timestampLe: Integer, subsystem: String, parameter: String, limit: Integer): [{
            timestamp: Integer!
            subsystem: String!
            parameter: String!
            value: String!
        }]
    }

Each of the query arguments acts as a filter for the database query:

    - timestampGe - Return entries with timestamps occurring on or after the given value
    - timestampLe - Return entries with timestamps occurring on or before the given value
    - subsystem - Return entries which match the given subsystem name
    - parameter - Return entries which match the given parameter name
    - limit - Return only the first `n` entries found

Note: ``timestampGe`` and ``timestampLe`` can be combined to create a timestamp selection range.
For example, entries with timestamps after ``1000``, but before ``5000``.

Saving Results for Later Processing
-----------------------------------

Immediate, large query results might consume more downlink bandwidth than is allowable.
Alternatively, downlink and uplink could be asynchronous from each other.

In this case, we can use the ``routedTelemetry`` query to write our results to an on-system file.
This way, we can choose the specific time at which to downlink the results using the
:doc:`file transfer service <file>`. Additionally, by default, the output file will be in a
compressed format, reducing the amount of data which needs to be transferred.

The query has the following schema::

    query {
        telemetry(timestampGe: Integer, timestampLe: Integer, subsystem: String, parameter: String, output: String!, compress: Boolean = true): String! 
    }

The ``output`` argument specifies the output file to write the query results to. It may be a relative or absolute path.

The ``compress`` argument specifies whether the service should compress the output file after writing the results to it.

The other arguments are the same as in the ``telemetry`` query.

The query will return a single field echoing the file that was written to.
If the ``compress`` argument is true (which is the default), then the result will be the output file name suffixed with ".tar.gz" to indicate
that the file was compressed using `Gzip <https://www.gnu.org/software/gzip/manual/gzip.html>`__.

The results file will contain an array of database entries in JSON format.
This matches the return fields of the ``telemetry`` query.

Adding Entries to the Database
------------------------------

The ``insert`` mutation can be used to add an entry to the telemetry database.

It has the following schema::

    mutation {
        insert(timestamp: Integer, subsystem: String!, parameter: String!, value: String!): {
            success: Boolean!,
            errors: String!
        }
    }

The ``timestamp`` argument is optional. If it is not specified, one will be generated based on the current system time,
in milliseconds.

Limitations
~~~~~~~~~~~

The generated timestamp value will be the current system time in milliseconds.
The database uses the combination of ``timestamp``, ``subsystem``, and ``parameter`` as the primary key.
This primary key must be unique for each entry.

    - As a result, any one subsystem parameter may not be logged more than once per millisecond.

Adding Entries to the Database Asynchronously
---------------------------------------------

If you would like to add many entries to the database quickly, and don't care about verifying that the request
was successful, the service's direct UDP port may be used.
This UDP port is configured with the ``direct_port`` value in the system's ``config.toml`` file.

Insert requests should be sent as individual UDP messages in JSON format.

The requests have the following schema::

    {
        "timestamp": Integer,
        "subsystem": String!,
        "parameter": String!,
        "value": String!,
    }

The ``timestamp`` argument is optional (one will be generated based on the current system time), but the other parameters are all required.

For example::

    {
        "subsystem": "eps",
        "parameter": "voltage",
        "value": "3.5"
    }

Limitations
~~~~~~~~~~~

The generated timestamp value will be the current system time in milliseconds.
The database uses the combination of ``timestamp``, ``subsystem``, and ``parameter`` as the primary key.
This primary key must be unique for each entry.

    - As a result, any one subsystem parameter may not be logged more than once per millisecond.

This asynchronous method sends requests to the telemetry database service much more quickly than time needed for the
service to process each request. The service's direct UDP socket buffer can store up to 256 packets at a time.

    - As a result, no more than 256 messages should be sent (from any and all sources) using this direct method in the time
      period required for the service to process them (this can be calculated by multiplying 256 by the amount of time required
      to process a single message. See the `Benchmark`_ section for more information).

The service processes requests from both the direct UDP method and the traditional GraphQL method one at a time,
rather than simultaneously.

    - As a result, if the service is receiving requests from both methods at the same time, the time period required
      to process 256 direct UDP messages should be doubled.

Removing Entries from the Database
----------------------------------

The ``delete`` mutation can be used to remove a selection of entries from the telemetry database.

It has the following schema::

    mutation {
        delete(timestampGe: Integer, timestampLe: Integer, subsystem: String, parameter: String): [{
            success: Boolean!,
            errors: String!,
            entriesDeleted: Integer
        }]
    }

Each of the mutation arguments acts as a filter for the database query:

    - timestampGe - Delete entries with timestamps occurring on or after the given value
    - timestampLe - Delete entries with timestamps occurring on or before the given value
    - subsystem - Delete entries which match the given subsystem name
    - parameter - Delete entries which match the given parameter name

The mutation has the following response fields:

    - success - Indicates whether the delete operation was successful
    - errors - Any errors encountered by the delete operation
    - entriesDeleted - The number of entries deleted by the operation
