(function() {
	if (typeof(Oculus) === 'undefined') {
		window.Oculus = {};
	}

	var Utils = Oculus.Utils = {};

	var inherits = Utils.inherits = function(child, parent) {
		var Surrogate = function () { this.constructor = child; };
		Surrogate.prototype = parent.prototype;
		child.prototype = new Surrogate();
	};

	var calcVec = Utils.calcVec = function (startPos, endPos, speed) {
		// TODO: determine speed vector based on location and base position
		var xDist = (startPos[0] - endPos[0]);
		var yDist = (startPos[1] - endPos[1]);

		var theta = Math.atan(yDist / xDist);
		var xVel = speed * Math.cos(theta);
		var yVel = speed  * Math.sin(theta);
		if (startPos[0] > endPos[0]){ yVel *= -1; xVel *= -1;}
		// if (startPos[1] > endPos[1]){ yVel *= -1;}
		// debugger;
		// var deg = 2 * Math.PI * Math.random();
		return [xVel, yVel];
		// return scale([Math.sin(deg), Math.cos(deg)], length);
	};

	// // Find distance between two points.
	// var dir = Utils.dir = function (vec) {
	// 	var norm = Utils.norm(vec);
	// 	return Utils.scale(vec, 1 / norm);
	// };
	//
	// Find distance between two points.
	var dist = Utils.dist = function (pos1, pos2) {
		return Math.sqrt(
			Math.pow(pos1[0] - pos2[0], 2) + Math.pow(pos1[1] - pos2[1], 2)
		);
	};
	//
	// // Find the length of the vector.
	// var norm = Utils.norm = function (vec) {
	// 	return Utils.dist([0, 0], vec);
	// };
	//
	// // Scale the length of a vector by the given amount.
	// var scale = Utils.scale = function (vec, m) {
	// 	return [vec[0] * m, vec[1] * m];
	// };

	// var calcVec = Utils.calcVec = function (pos, target, speed){
	// 	debugger;
	// };

})();
