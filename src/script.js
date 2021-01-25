// Gamepad socket scripting
// https://github.com/luser/gamepadtest/blob/master/gamepadtest.js
var haveEvents = "GamepadEvent" in window;
var haveWebkitEvents = "WebKitGamepadEvent" in window;
var controllers = {};
var rAF = window.requestAnimationFrame;

// http://jsfiddle.net/m1erickson/CtsY3/
var startTime, now, then;
then = Date.now();
var fpsInterval = 1000 / 5; // 5fps

// Create WebSocket connection.
const socket = new WebSocket(
  "ws://" + window.location.host + window.location.pathname + "sock"
);
var haveSocket = false;

// Connection opened
socket.addEventListener("open", function (event) {
  haveSocket = true;
  console.log("WebSocket opened: ", event);
});

socket.addEventListener("close", (event) => {
  haveSocket = false;
  console.log("WebSocket closed: ", event);
});

// Listen for possible errors
socket.addEventListener("error", function (event) {
  console.log("WebSocket error: ", event);
});

function connecthandler(e) {
  addgamepad(e.gamepad);
}
function addgamepad(gamepad) {
  controllers[gamepad.index] = gamepad;
  var d = document.createElement("div");
  d.setAttribute("id", "controller" + gamepad.index);
  var t = document.createElement("h1");
  t.appendChild(document.createTextNode("gamepad: " + gamepad.id));
  d.appendChild(t);
  var b = document.createElement("div");
  b.className = "buttons";
  for (var i = 0; i < gamepad.buttons.length; i++) {
    var e = document.createElement("span");
    e.className = "button";
    //e.id = "b" + i;
    e.innerHTML = i;
    b.appendChild(e);
  }
  d.appendChild(b);
  var a = document.createElement("div");
  a.className = "axes";
  for (i = 0; i < gamepad.axes.length; i++) {
    e = document.createElement("meter");
    e.className = "axis";
    //e.id = "a" + i;
    e.setAttribute("min", "-1");
    e.setAttribute("max", "1");
    e.setAttribute("value", "0");
    e.innerHTML = i;
    a.appendChild(e);
  }
  d.appendChild(a);
  document.getElementById("start").style.display = "none";
  document.body.appendChild(d);
  rAF(updateStatus);
}

function disconnecthandler(e) {
  removegamepad(e.gamepad);
}

function removegamepad(gamepad) {
  var d = document.getElementById("controller" + gamepad.index);
  document.body.removeChild(d);
  delete controllers[gamepad.index];
}

function updateStatus() {
  scangamepads();
  for (j in controllers) {
    var controller = controllers[j];
    var d = document.getElementById("controller" + j);
    var buttons = d.getElementsByClassName("button");
    for (var i = 0; i < controller.buttons.length; i++) {
      var b = buttons[i];
      var val = controller.buttons[i];
      var pressed = val == 1.0;
      var touched = false;
      if (typeof val == "object") {
        pressed = val.pressed;
        if ("touched" in val) {
          touched = val.touched;
        }
        val = val.value;
      }
      var pct = Math.round(val * 100) + "%";
      b.style.backgroundSize = pct + " " + pct;
      b.className = "button";
      if (pressed) {
        b.className += " pressed";
      }
      if (touched) {
        b.className += " touched";
      }
    }

    var axes = d.getElementsByClassName("axis");
    for (var i = 0; i < controller.axes.length; i++) {
      var a = axes[i];
      a.innerHTML = i + ": " + controller.axes[i].toFixed(4);
      a.setAttribute("value", controller.axes[i]);
    }

    now = Date.now();
    var elapsed = now - then;

    if (haveSocket && elapsed > fpsInterval) {
      // Encode Gamepad object.
      console.log("socket send!");
      socket.send(
        JSON.stringify({
          id: controller.id,
          buttons: controller.buttons.map((v) => ({
            value: v.value,
            pressed: v.pressed,
          })),
          axes: controller.axes,
        })
      );
    }
  }
  rAF(updateStatus);
}

function scangamepads() {
  var gamepads = navigator.getGamepads
    ? navigator.getGamepads()
    : navigator.webkitGetGamepads
    ? navigator.webkitGetGamepads()
    : [];
  for (var i = 0; i < gamepads.length; i++) {
    if (gamepads[i] && gamepads[i].index in controllers) {
      controllers[gamepads[i].index] = gamepads[i];
    }
  }
}

if (haveEvents) {
  window.addEventListener("gamepadconnected", connecthandler);
  window.addEventListener("gamepaddisconnected", disconnecthandler);
} else if (haveWebkitEvents) {
  window.addEventListener("webkitgamepadconnected", connecthandler);
  window.addEventListener("webkitgamepaddisconnected", disconnecthandler);
} else {
  setInterval(scangamepads, 500);
}
