#import "NGSMetalView.h"

@implementation NGSMetalView

- (instancetype)initWithFrame:(CGRect)frame
{
    if ((self = [self initWithFrame:frame device:MTLCreateSystemDefaultDevice()])) {
        return self;
    }
    return nil;
}

- (void)drawRect:(CGRect)rect
{
    id <CAMetalDrawable> drawable = [self currentDrawable];

}

@end
