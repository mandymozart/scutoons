# Sample Triggers

Slices can trigger audio samples. Samples are defined in the audio section of the slice.


Recommended adoptions to the data structure of a chapter's json file.
```json
{
    //...
  "audio":
    //...
    "samples": [
        {
            "id": "sample_01",
            "trigger": {
                "type": "scrollInView", // inView | click | hover,
                "gate": false, // true | false,
                "exclusive": false, // true | false (mutes other audio),
                "mode": "mono", // mono | poly (can be triggered more than once if set to poly, otherwise the previous trigger will be debounced)
            },
            "path": "path/to/sample.mp3",
            "delay": 1000,
            "volume": 1,
            "pan": 0,
            "pitch": 0,
            "loop": false,
            "envelope": {
                "attack": 1000,
                "sustain": 1000,
                "release": 1000,
            }
        }
        //...
    ]
}
```

## Implementation

Make sure to cache audio samples to prevent multiple requests. 

### Preloader

It should preload all audio alongside the slices loader, so synchronize those two. A slice is not loaded if the audio is not fully loaded. 