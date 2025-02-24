import test from 'tape';
import sjs from 'simhash-js';
import * as minhash from 'minhash';
import  _ from 'lodash';

const myInfoChrome =
{
  "products": {
    "identification": {
      "data": {
        "browserDetails": {
          "browserName": "Chrome",
          "browserMajorVersion": "128",
          "browserFullVersion": "128.0.0",
          "os": "Mac OS X",
          "osVersion": "10.15.7",
          "device": "Other",
          "userAgent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36"
        },
      },
    },
    "rootApps": {
      "data": {
        "result": false
      }
    },
    "emulator": {
      "data": {
        "result": false
      }
    },
    "proxy": {
      "data": {
        "result": false
      }
    },
    "incognito": {
      "data": {
        "result": false
      }
    },
    "tampering": {
      "data": {
        "result": false,
        "anomalyScore": 0,
        "antiDetectBrowser": false
      }
    },
    "clonedApp": {
      "data": {
        "result": false
      }
    },
    "factoryReset": {
      "data": {
        "time": "1970-01-01T00:00:00Z",
        "timestamp": 0
      }
    },
    "jailbroken": {
      "data": {
        "result": false
      }
    },
    "frida": {
      "data": {
        "result": false
      }
    },
    "privacySettings": {
      "data": {
        "result": false
      }
    },
    "virtualMachine": {
      "data": {
        "result": false
      }
    },
    "rawDeviceAttributes": {
      "data": {
        "architecture": {
          "value": 255
        },
        "audio": {
          "value": 124.04347657808103
        },
        "audioBaseLatency": {
          "value": -2
        },
        "canvas": {
          "value": {
            "Geometry": "70a0a884fada8bac5f9e42250271aaf4",
            "Text": "bc08138519802f8fef528a65d64e5048",
            "Winding": true
          }
        },
        "colorDepth": {
          "value": 24
        },
        "colorGamut": {
          "value": "srgb"
        },
        "contrast": {
          "value": 0
        },
        "cookiesEnabled": {
          "value": true
        },
        "cpuClass": {},
        "deviceMemory": {
          "value": 8
        },
        "domBlockers": {},
        "emoji": {
          "value": {
            "bottom": 28,
            "font": "Times",
            "height": 18.5,
            "left": 8,
            "right": 1288,
            "top": 9.5,
            "width": 1280,
            "x": 8,
            "y": 9.5
          }
        },
        "fontPreferences": {
          "value": {
            "apple": 73.78125,
            "default": 73.78125,
            "min": 4.6171875,
            "mono": 66.53125,
            "sans": 72.0078125,
            "serif": 73.78125,
            "system": 73.0390625
          }
        },
        "fonts": {
          "value": [
            "Arial Unicode MS",
            "Gill Sans",
            "Helvetica Neue",
            "Menlo"
          ]
        },
        "forcedColors": {
          "value": false
        },
        "hardwareConcurrency": {
          "value": 6
        },
        "hdr": {
          "value": false
        },
        "indexedDB": {
          "value": true
        },
        "invertedColors": {},
        "languages": {
          "value": [
            [
              "en-US"
            ]
          ]
        },
        "localStorage": {
          "value": true
        },
        "math": {
          "value": "5963cfe25fe61d0bbd7b4920bc602dc8"
        },
        "mathML": {
          "value": {
            "bottom": 29,
            "font": "Times",
            "height": 18.5,
            "left": 8,
            "right": 303.140625,
            "top": 10.5,
            "width": 295.140625,
            "x": 8,
            "y": 10.5
          }
        },
        "monochrome": {
          "value": 0
        },
        "openDatabase": {
          "value": false
        },
        "osCpu": {},
        "pdfViewerEnabled": {
          "value": true
        },
        "platform": {
          "value": "MacIntel"
        },
        "plugins": {
          "value": [
            {
              "description": "Portable Document Format",
              "mimeTypes": [
                {
                  "suffixes": "pdf",
                  "type": "application/pdf"
                },
                {
                  "suffixes": "pdf",
                  "type": "text/pdf"
                }
              ],
              "name": "PDF Viewer"
            },
            {
              "description": "Portable Document Format",
              "mimeTypes": [
                {
                  "suffixes": "pdf",
                  "type": "application/pdf"
                },
                {
                  "suffixes": "pdf",
                  "type": "text/pdf"
                }
              ],
              "name": "Chrome PDF Viewer"
            },
            {
              "description": "Portable Document Format",
              "mimeTypes": [
                {
                  "suffixes": "pdf",
                  "type": "application/pdf"
                },
                {
                  "suffixes": "pdf",
                  "type": "text/pdf"
                }
              ],
              "name": "Chromium PDF Viewer"
            },
            {
              "description": "Portable Document Format",
              "mimeTypes": [
                {
                  "suffixes": "pdf",
                  "type": "application/pdf"
                },
                {
                  "suffixes": "pdf",
                  "type": "text/pdf"
                }
              ],
              "name": "Microsoft Edge PDF Viewer"
            },
            {
              "description": "Portable Document Format",
              "mimeTypes": [
                {
                  "suffixes": "pdf",
                  "type": "application/pdf"
                },
                {
                  "suffixes": "pdf",
                  "type": "text/pdf"
                }
              ],
              "name": "WebKit built-in PDF"
            }
          ]
        },
        "privateClickMeasurement": {},
        "reducedMotion": {
          "value": false
        },
        "screenFrame": {
          "value": [
            20,
            0,
            0,
            0
          ]
        },
        "screenResolution": {
          "value": [
            1920,
            1080
          ]
        },
        "sessionStorage": {
          "value": true
        },
        "timezone": {
          "value": "America/Los_Angeles"
        },
        "touchSupport": {
          "value": {
            "maxTouchPoints": 0,
            "touchEvent": false,
            "touchStart": false
          }
        },
        "vendor": {
          "value": "Google Inc."
        },
        "vendorFlavors": {
          "value": [
            "chrome"
          ]
        },
        "webGlBasics": {
          "value": {
            "renderer": "WebKit WebGL",
            "rendererUnmasked": "ANGLE (Intel, ANGLE Metal Renderer: Intel(R) UHD Graphics 630, Unspecified Version)",
            "shadingLanguageVersion": "WebGL GLSL ES 1.0 (OpenGL ES GLSL ES 1.0 Chromium)",
            "vendor": "WebKit",
            "vendorUnmasked": "Google Inc. (Intel)",
            "version": "WebGL 1.0 (OpenGL ES 2.0 Chromium)"
          }
        },
        "webGlExtensions": {
          "value": {
            "contextAttributes": "6b1ed336830d2bc96442a9d76373252a",
            "extensionParameters": "22fad1bae5d96011ed8c710f66905ce2",
            "extensions": "2ec6a06c62e232a16e719d1738edbc16",
            "parameters": "7d0a67f7d85a024b06ef41337838aeca",
            "shaderPrecisions": "f223dfbcd580cf142da156d93790eb83",
            "unsupportedExtensions": []
          }
        }
      }
    },
    "highActivity": {
      "data": {
        "result": false
      }
    },
    "locationSpoofing": {
      "data": {
        "result": false
      }
    },
    "suspectScore": {
      "data": {
        "result": 0
      }
    },
    "remoteControl": {
      "data": {
        "result": false
      }
    },
    "velocity": {
      "data": {
        "distinctIp": {
          "intervals": {
            "5m": 1,
            "1h": 1,
            "24h": 1
          }
        },
        "distinctLinkedId": {},
        "distinctCountry": {
          "intervals": {
            "5m": 1,
            "1h": 1,
            "24h": 1
          }
        },
        "events": {
          "intervals": {
            "5m": 1,
            "1h": 1,
            "24h": 1
          }
        },
        "ipEvents": {
          "intervals": {
            "5m": 1,
            "1h": 1,
            "24h": 1
          }
        },
        "distinctIpByLinkedId": {},
        "distinctVisitorIdByLinkedId": {}
      }
    },
    "developerTools": {
      "data": {
        "result": false
      }
    }
  }
};

const myInfoFirefox = 
{
  "products": {
    "identification": {
      "data": {
        "visitorId": "zwxxJymQFQFjTCmjR0iJ",
        "requestId": "1735285063381.DqNoLZ",
        "browserDetails": {
          "browserName": "Firefox",
          "browserMajorVersion": "133",
          "browserFullVersion": "133.0",
          "os": "Mac OS X",
          "osVersion": "10.15",
          "device": "Other",
          "userAgent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:133.0) Gecko/20100101 Firefox/133.0"
        },
        "incognito": false,
        "ip": "98.232.83.177",
        "ipLocation": {
          "accuracyRadius": 20,
          "latitude": 46.9754,
          "longitude": -123.8157,
          "postalCode": "98520",
          "timezone": "America/Los_Angeles",
          "city": {
            "name": "Aberdeen"
          },
          "country": {
            "code": "US",
            "name": "United States"
          },
          "continent": {
            "code": "NA",
            "name": "North America"
          },
          "subdivisions": [
            {
              "isoCode": "WA",
              "name": "Washington"
            }
          ]
        },
        "timestamp": 1735285063388,
        "time": "2024-12-27T07:37:43Z",
        "url": "https://fingerprint.com/",
        "tag": {
          "referrerLink": null
        },
        "confidence": {
          "score": 1,
          "revision": "v1.1"
        },
        "visitorFound": false,
        "firstSeenAt": {
          "global": "2024-12-27T07:37:43.388Z",
          "subscription": "2024-12-27T07:37:43.388Z"
        },
        "lastSeenAt": {
          "global": null,
          "subscription": null
        }
      }
    },
    "botd": {
      "data": {
        "bot": {
          "result": "notDetected"
        },
        "meta": {
          "referrerLink": null
        },
        "url": "https://fingerprint.com/",
        "ip": "98.232.83.177",
        "time": "2024-12-27T07:37:43.412Z",
        "userAgent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:133.0) Gecko/20100101 Firefox/133.0",
        "requestId": "1735285063381.DqNoLZ"
      }
    },
    "rootApps": {
      "data": {
        "result": false
      }
    },
    "emulator": {
      "data": {
        "result": false
      }
    },
    "ipInfo": {
      "data": {
        "v4": {
          "address": "98.232.83.177",
          "geolocation": {
            "accuracyRadius": 20,
            "latitude": 46.9754,
            "longitude": -123.8157,
            "postalCode": "98520",
            "timezone": "America/Los_Angeles",
            "city": {
              "name": "Aberdeen"
            },
            "country": {
              "code": "US",
              "name": "United States"
            },
            "continent": {
              "code": "NA",
              "name": "North America"
            },
            "subdivisions": [
              {
                "isoCode": "WA",
                "name": "Washington"
              }
            ]
          },
          "asn": {
            "asn": "7922",
            "name": "COMCAST-7922",
            "network": "98.232.0.0/13"
          },
          "datacenter": {
            "result": false,
            "name": ""
          }
        }
      }
    },
    "ipBlocklist": {
      "data": {
        "result": false,
        "details": {
          "emailSpam": false,
          "attackSource": false
        }
      }
    },
    "tor": {
      "data": {
        "result": false
      }
    },
    "vpn": {
      "data": {
        "result": false,
        "confidence": "high",
        "originTimezone": "America/Los_Angeles",
        "originCountry": "unknown",
        "methods": {
          "timezoneMismatch": false,
          "publicVPN": false,
          "auxiliaryMobile": false,
          "osMismatch": false,
          "relay": false
        }
      }
    },
    "proxy": {
      "data": {
        "result": false
      }
    },
    "incognito": {
      "data": {
        "result": false
      }
    },
    "tampering": {
      "data": {
        "result": false,
        "anomalyScore": 0,
        "antiDetectBrowser": false
      }
    },
    "clonedApp": {
      "data": {
        "result": false
      }
    },
    "factoryReset": {
      "data": {
        "time": "1970-01-01T00:00:00Z",
        "timestamp": 0
      }
    },
    "jailbroken": {
      "data": {
        "result": false
      }
    },
    "frida": {
      "data": {
        "result": false
      }
    },
    "privacySettings": {
      "data": {
        "result": false
      }
    },
    "virtualMachine": {
      "data": {
        "result": false
      }
    },
    "rawDeviceAttributes": {
      "data": {
        "architecture": {
          "value": 255
        },
        "audio": {
          "value": 35.749972093850374
        },
        "audioBaseLatency": {
          "value": -2
        },
        "canvas": {
          "value": {
            "Geometry": "1dc68a668ae41add6591624fcfa5c263",
            "Text": "80fca89f4d9b364ded995eea557e7afb",
            "Winding": true
          }
        },
        "colorDepth": {
          "value": 30
        },
        "colorGamut": {
          "value": "srgb"
        },
        "contrast": {
          "value": 0
        },
        "cookiesEnabled": {
          "value": true
        },
        "cpuClass": {},
        "deviceMemory": {},
        "domBlockers": {},
        "emoji": {
          "value": {
            "bottom": 28,
            "font": "serif",
            "height": 16,
            "left": 8,
            "right": 1288,
            "top": 12,
            "width": 1280,
            "x": 8,
            "y": 12
          }
        },
        "fontPreferences": {
          "value": {
            "apple": 147.5833282470703,
            "default": 147.5833282470703,
            "min": 9.23333740234375,
            "mono": 133.1666717529297,
            "sans": 144.01666259765625,
            "serif": 147.5833282470703,
            "system": 145.98333740234375
          }
        },
        "fonts": {
          "value": [
            "Arial Unicode MS",
            "Gill Sans",
            "Helvetica Neue",
            "Menlo"
          ]
        },
        "forcedColors": {
          "value": false
        },
        "hardwareConcurrency": {
          "value": 6
        },
        "hdr": {
          "value": false
        },
        "indexedDB": {
          "value": true
        },
        "invertedColors": {},
        "languages": {
          "value": [
            [
              "en-US"
            ],
            [
              "en-US",
              "en"
            ]
          ]
        },
        "localStorage": {
          "value": true
        },
        "math": {
          "value": "f604fac7dd981bf0f0ddfba24b202989"
        },
        "mathML": {
          "value": {
            "bottom": 29,
            "font": "serif",
            "height": 16,
            "left": 8,
            "right": 301.8833312988281,
            "top": 13,
            "width": 293.8833312988281,
            "x": 8,
            "y": 13
          }
        },
        "monochrome": {
          "value": 0
        },
        "openDatabase": {
          "value": false
        },
        "osCpu": {
          "value": "Intel Mac OS X 10.15"
        },
        "pdfViewerEnabled": {
          "value": true
        },
        "platform": {
          "value": "MacIntel"
        },
        "plugins": {
          "value": [
            {
              "description": "Portable Document Format",
              "mimeTypes": [
                {
                  "suffixes": "pdf",
                  "type": "application/pdf"
                },
                {
                  "suffixes": "pdf",
                  "type": "text/pdf"
                }
              ],
              "name": "PDF Viewer"
            },
            {
              "description": "Portable Document Format",
              "mimeTypes": [
                {
                  "suffixes": "pdf",
                  "type": "application/pdf"
                },
                {
                  "suffixes": "pdf",
                  "type": "text/pdf"
                }
              ],
              "name": "Chrome PDF Viewer"
            },
            {
              "description": "Portable Document Format",
              "mimeTypes": [
                {
                  "suffixes": "pdf",
                  "type": "application/pdf"
                },
                {
                  "suffixes": "pdf",
                  "type": "text/pdf"
                }
              ],
              "name": "Chromium PDF Viewer"
            },
            {
              "description": "Portable Document Format",
              "mimeTypes": [
                {
                  "suffixes": "pdf",
                  "type": "application/pdf"
                },
                {
                  "suffixes": "pdf",
                  "type": "text/pdf"
                }
              ],
              "name": "Microsoft Edge PDF Viewer"
            },
            {
              "description": "Portable Document Format",
              "mimeTypes": [
                {
                  "suffixes": "pdf",
                  "type": "application/pdf"
                },
                {
                  "suffixes": "pdf",
                  "type": "text/pdf"
                }
              ],
              "name": "WebKit built-in PDF"
            }
          ]
        },
        "privateClickMeasurement": {},
        "reducedMotion": {
          "value": false
        },
        "screenFrame": {
          "value": [
            20,
            0,
            0,
            0
          ]
        },
        "screenResolution": {
          "value": [
            1920,
            1080
          ]
        },
        "sessionStorage": {
          "value": true
        },
        "timezone": {
          "value": "America/Los_Angeles"
        },
        "touchSupport": {
          "value": {
            "maxTouchPoints": 0,
            "touchEvent": false,
            "touchStart": false
          }
        },
        "vendor": {
          "value": ""
        },
        "vendorFlavors": {
          "value": []
        },
        "webGlBasics": {
          "value": {
            "renderer": "Radeon R9 200 Series, or similar",
            "rendererUnmasked": "",
            "shadingLanguageVersion": "WebGL GLSL ES 1.0",
            "vendor": "Mozilla",
            "vendorUnmasked": "",
            "version": "WebGL 1.0"
          }
        },
        "webGlExtensions": {
          "value": {
            "contextAttributes": "48e630ae802ba2dc7ed36e8728edf4fc",
            "extensionParameters": "04af766f0af52fa872ee06462d32d8fa",
            "extensions": "1a3d1ad9553685cd1c72a387e4f2eb24",
            "parameters": "17df6088cd82d6efe2f80ce643eb4707",
            "shaderPrecisions": "1aabf55167f5f14457db90ec2b1a88f0",
            "unsupportedExtensions": []
          }
        }
      }
    },
    "highActivity": {
      "data": {
        "result": false
      }
    },
    "locationSpoofing": {
      "data": {
        "result": false
      }
    },
    "suspectScore": {
      "data": {
        "result": 0
      }
    },
    "remoteControl": {
      "data": {
        "result": false
      }
    },
    "velocity": {
      "data": {
        "distinctIp": {
          "intervals": {
            "5m": 1,
            "1h": 1,
            "24h": 1
          }
        },
        "distinctLinkedId": {},
        "distinctCountry": {
          "intervals": {
            "5m": 1,
            "1h": 1,
            "24h": 1
          }
        },
        "events": {
          "intervals": {
            "5m": 1,
            "1h": 1,
            "24h": 1
          }
        },
        "ipEvents": {
          "intervals": {
            "5m": 1,
            "1h": 3,
            "24h": 3
          }
        },
        "distinctIpByLinkedId": {},
        "distinctVisitorIdByLinkedId": {}
      }
    },
    "developerTools": {
      "data": {
        "result": false
      }
    }
  }
};





test('basic simhash v minhash', async(t) => {
  const simhash = new sjs.SimHash();
  //const a = "This is a test of the Emergency Blogcast System";
  //const b = "This is a testy of the Emergency Blogcast System";
  const a = 'Opera/9.80 (Macintosh; Intel Mac OS X; U; en) Presto/2.2.15 Version/10.00';
  const b = 'Opera/9.09 (Macintosh; Intel Mac OS X; U; sp) Presto/2.3.24 Version/9.90';
  const c = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko)';

  const x = simhash.hash(a);
  const y = simhash.hash(b);
  const z = simhash.hash(c);

  const s12 = sjs.Comparator.similarity(x, y); 
  const s13 = sjs.Comparator.similarity(x, z); 
  console.log('\n',x.toString(2),'\n',y.toString(2),'\n',z.toString(2),s12,s13);

  const s1 = a.split(' ');
  const s2 = b.split(' ');
  const s3 = c.split(' ');

  const m1 = new minhash.Minhash();
  const m2 = new minhash.Minhash();
  const m3 = new minhash.Minhash();

  s1.map(function(w) { m1.update(w) });
  s2.map(function(w) { m2.update(w) });
  s3.map(function(w) { m3.update(w) });

  const mh1 = m1.hash(a);
  const mh2 = m1.hash(b);
  const mh3 = m1.hash(c);
  console.log(mh1.toString(2), mh2.toString(2), mh3.toString(2));

  const m12 = m1.jaccard(m2);
  const m13 = m1.jaccard(m3);
  console.log(m12, m13, s12, s13);

  const index = new minhash.LshIndex();
  index.insert('m1', m1);
  index.insert('m2', m2);
  const matches = index.query(m1);
  
  console.log('Jaccard similarity >= 0.5 to m1:', matches);


  t.end();
});

function extractValues(input) {
  const values = [];

  function traverse(obj) {
    if (_.isPlainObject(obj)) {
      Object.values(obj).forEach(traverse);
    } else {
      values.push(obj);
    }
  }

  traverse(input);

  return _.flattenDeep(values).join(', ');
}

test('simhash of example browser data', async(t) => {
  const simhash = new sjs.SimHash();
  const chromeValues = extractValues(myInfoChrome);
  const firefoxValues = extractValues(myInfoFirefox); 
  const chromeHash = simhash.hash(chromeValues);
  const firefoxHash = simhash.hash(firefoxValues);

  myInfoChrome.products.identification.data.osVersion = "11.5.2";
  const newChromeValues = extractValues(myInfoChrome);
  const newChromeHash = simhash.hash(chromeValues);

  console.log(chromeHash.toString(2));
  console.log(newChromeHash.toString(2));
  console.log(firefoxHash.toString(2));

  
  t.end();
});
