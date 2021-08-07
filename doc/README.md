# Sensor Communication Protocol

## Requirements

- The protocol MUST support unidirectional, asynchronous message transfer from the sensor to the OxPad.
- The sensor MUST resync in the event of garbled data.
- The protocol MAY in future need to support commands from the OxPad to the sensor.

## Message format

This is based on HDLC, but altered to make it more robust:

 - The start sequence and checksum are longer, to make transmission errors less likely.
 - Unnecessary flags are eliminated.

This leaves a package format of:

```
Start:  AAAA5555 (4 octets)
Recipient: 1 octet
Counter: 1 octet
Payload length excluding header: 2 octets
Payload: zero or more octets
Checksum: 4 octets, including start
```

#### Start
This is a fixed pattern, used by the receiver to identify the start of the message.

#### Recipient
This separates messages into different streams, e.g. command and control from primary optical data.

#### Counter
This is a message counter within a stream.  It allows detection of missing messages.  The counter wraps around.

#### Payload Length
The length of the inner packet in octets, in little endian format.  This does not need to be compatible with legacy, big endian protocols.

#### Payload
The payload may be an arbitrary sequence of octets.

For Rust data structures, a straightforward encoding of primitive types to bytes is enough.  The encoding must be consistent, i.e. without padding bytes.  Given that the memory  representation of a typical non-aligned data structure contains padding, the encoding should be such as that of bincode, with the padding squeezed out.

The data is frobnicated with the conter as seed, to make it unlikely that the payload data can consistently interfere with synchonisation.

#### Checksum
We recommend CRC32C.


## Messages

Messages are bincode encoded versions of the following:

```
#[derive(Copy)]
enum SensorMessage {
    SensorHealth(SensorHealthMessage),
    PrimaryOpticalMessage(PrimaryOpticalMessage),
    EnvironmentalMessage(EnvironmentalMessage),
}

#[derive(Copy)]
struct SensorHealthMessage { TODO: Sensor version and any alertable conditions, such as over-temperature. }

#[derive(Copy)]
struct PrimaryOpticalMessage { TODO: LED inntensities. }

#[derive(Copy)]
struct EnvironmentalMessage { TODO: Thermometer, accelerometer etc. }
```
