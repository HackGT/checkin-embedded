"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : new P(function (resolve) { resolve(result.value); }).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __generator = (this && this.__generator) || function (thisArg, body) {
    var _ = { label: 0, sent: function() { if (t[0] & 1) throw t[1]; return t[1]; }, trys: [], ops: [] }, f, y, t, g;
    return g = { next: verb(0), "throw": verb(1), "return": verb(2) }, typeof Symbol === "function" && (g[Symbol.iterator] = function() { return this; }), g;
    function verb(n) { return function (v) { return step([n, v]); }; }
    function step(op) {
        if (f) throw new TypeError("Generator is already executing.");
        while (_) try {
            if (f = 1, y && (t = op[0] & 2 ? y["return"] : op[0] ? y["throw"] || ((t = y["return"]) && t.call(y), 0) : y.next) && !(t = t.call(y, op[1])).done) return t;
            if (y = 0, t) op = [op[0] & 2, t.value];
            switch (op[0]) {
                case 0: case 1: t = op; break;
                case 4: _.label++; return { value: op[1], done: false };
                case 5: _.label++; y = op[1]; op = [0]; continue;
                case 7: op = _.ops.pop(); _.trys.pop(); continue;
                default:
                    if (!(t = _.trys, t = t.length > 0 && t[t.length - 1]) && (op[0] === 6 || op[0] === 2)) { _ = 0; continue; }
                    if (op[0] === 3 && (!t || (op[1] > t[0] && op[1] < t[3]))) { _.label = op[1]; break; }
                    if (op[0] === 6 && _.label < t[1]) { _.label = t[1]; t = op; break; }
                    if (t && _.label < t[2]) { _.label = t[2]; _.ops.push(op); break; }
                    if (t[2]) _.ops.pop();
                    _.trys.pop(); continue;
            }
            op = body.call(thisArg, _);
        } catch (e) { op = [6, e]; y = 0; } finally { f = t = 0; }
        if (op[0] & 5) throw op[1]; return { value: op[0] ? op[1] : void 0, done: true };
    }
};
var _this = this;
function setupButtonHandlers(className, handler) {
    var _this = this;
    var buttons = document.getElementsByClassName(className);
    if (buttons.length === 0) {
        console.warn("Didn't find any buttons for class: " + className);
    }
    for (var i = 0; i < buttons.length; i++) {
        buttons[i].addEventListener("click", function (e) { return __awaiter(_this, void 0, void 0, function () {
            var button, deviceUsername;
            return __generator(this, function (_a) {
                switch (_a.label) {
                    case 0:
                        button = e.target;
                        deviceUsername = button.parentElement.dataset.username;
                        if (!deviceUsername) return [3 /*break*/, 2];
                        button.disabled = true;
                        return [4 /*yield*/, handler(deviceUsername)];
                    case 1:
                        _a.sent();
                        button.disabled = false;
                        _a.label = 2;
                    case 2: return [2 /*return*/];
                }
            });
        }); });
    }
}
function makeButtonRequest(url, username) {
    return __awaiter(this, void 0, void 0, function () {
        return __generator(this, function (_a) {
            return [2 /*return*/, fetch(url, {
                    method: "POST",
                    credentials: "include",
                    headers: {
                        "Content-Type": "application/json"
                    },
                    body: JSON.stringify({ username: username })
                }).then(function (response) { return response.json(); })];
        });
    });
}
setupButtonHandlers("action-authorize", function (id) { return __awaiter(_this, void 0, void 0, function () {
    var response;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0: return [4 /*yield*/, makeButtonRequest("/api/device/authorize", id)];
            case 1:
                response = _a.sent();
                if (response.success) {
                    window.location.reload();
                }
                else {
                    alert(response.error + " (" + (response.details || "No details") + ")");
                }
                return [2 /*return*/];
        }
    });
}); });
setupButtonHandlers("action-reject", function (id) { return __awaiter(_this, void 0, void 0, function () {
    var response;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0: return [4 /*yield*/, makeButtonRequest("/api/device/reject", id)];
            case 1:
                response = _a.sent();
                if (response.success) {
                    window.location.reload();
                }
                else {
                    alert(response.error + " (" + (response.details || "No details") + ")");
                }
                return [2 /*return*/];
        }
    });
}); });
setupButtonHandlers("action-force-renew", function (id) { return __awaiter(_this, void 0, void 0, function () {
    var response;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0: return [4 /*yield*/, makeButtonRequest("/api/device/force-renew", id)];
            case 1:
                response = _a.sent();
                if (response.success) {
                    window.location.reload();
                }
                else {
                    alert(response.error + " (" + (response.details || "No details") + ")");
                }
                return [2 /*return*/];
        }
    });
}); });
setupButtonHandlers("action-delete", function (id) { return __awaiter(_this, void 0, void 0, function () {
    var response;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0: return [4 /*yield*/, makeButtonRequest("/api/device/delete", id)];
            case 1:
                response = _a.sent();
                if (response.success) {
                    window.location.reload();
                }
                else {
                    alert(response.error + " (" + (response.details || "No details") + ")");
                }
                return [2 /*return*/];
        }
    });
}); });
setupButtonHandlers("action-rename", function (id) { return __awaiter(_this, void 0, void 0, function () {
    var name, response;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                name = prompt("New device name:");
                if (!name)
                    return [2 /*return*/];
                return [4 /*yield*/, fetch("/api/device/rename", {
                        method: "POST",
                        credentials: "include",
                        headers: {
                            "Content-Type": "application/json"
                        },
                        body: JSON.stringify({ username: id, name: name })
                    }).then(function (response) { return response.json(); })];
            case 1:
                response = _a.sent();
                if (response.success) {
                    window.location.reload();
                }
                else {
                    alert(response.error + " (" + (response.details || "No details") + ")");
                }
                return [2 /*return*/];
        }
    });
}); });
var selects = document.getElementsByClassName("tag-select");
for (var i = 0; i < selects.length; i++) {
    selects[i].addEventListener("change", function (e) { return __awaiter(_this, void 0, void 0, function () {
        var select, deviceUsername, response;
        return __generator(this, function (_a) {
            switch (_a.label) {
                case 0:
                    select = e.target;
                    deviceUsername = select.dataset.username;
                    if (!deviceUsername) return [3 /*break*/, 2];
                    select.disabled = true;
                    return [4 /*yield*/, fetch("/api/device/set-tag", {
                            method: "POST",
                            credentials: "include",
                            headers: {
                                "Content-Type": "application/json"
                            },
                            body: JSON.stringify({ username: deviceUsername, tag: select.value })
                        }).then(function (response) { return response.json(); })];
                case 1:
                    response = _a.sent();
                    if (!response.success) {
                        alert(response.error + " (" + (response.details || "No details") + ")");
                    }
                    select.disabled = false;
                    _a.label = 2;
                case 2: return [2 /*return*/];
            }
        });
    }); });
}
