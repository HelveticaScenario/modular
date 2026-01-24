poly is modeled after vcv (each module has 16 channels).

module channel use is affected by the channel count of signals used as params. if a sine oscillator has a freq of a module with 3 channels, that sine's output has 3 channels, for each feq. the channel count of a module is the max channel count of its params, and params with fewer channels are "cycled", which is `get channel n from signal with channel count m -> chan[n % m]`

argument multichannel expansion based on kabalsalat (arguments can take an array and that spreads to different channels of module)
eg: noise(['while', 'pink', 'brown']) makes a noise that uses 3 channels of its max 16, with the different colors used in each

questions
should all non-signal params be multichannel expandable? are there params that are just config params for a module, like "voice allocation strategy" and "channel count"? some params dont make sense as poly

should channel count be determined at patch update time, or dynamic within the lifecycle of a patch?

table stakes
- seq should allow stacks and haps with overlapping times

